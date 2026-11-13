//! Ely.by accounts: signed in with the Yggdrasil API at authserver.ely.by (a username or email
//! and a password), not through Microsoft. Their sessions play the game and join servers through
//! Ely.by's session server; no Minecraft purchase is checked, so such an account passes the
//! owner gate and may have the game downloaded for it.
//!
//! The access token is validated at every check and renewed with the client token when Ely.by
//! refuses it. The password is never kept.

use super::*;

const AUTH: &str = "https://authserver.ely.by/auth";
const SKINS: &str = "https://skinsystem.ely.by/skins";
const SIGN_IN_AGAIN_ELYBY: &str = "Your Ely.by sign-in expired. Sign in again.";
const SERVICE: &str = "Ely.by";
/// Ely.by tokens carry no expiry we can read; they are checked instead of timed.
const VALID_FOR: u64 = 365 * 24 * 3600;

fn post(path: &str, body: Value) -> Result<(u16, Value), String> {
    send(
        request_to("POST", &format!("{AUTH}/{path}")),
        Some(Body::Json(body)),
        SERVICE,
    )
}

/// The player-facing reason Ely.by refused a request.
fn refusal(v: &Value) -> String {
    let said = v.get("errorMessage").and_then(Value::as_str).unwrap_or("");
    if said.to_lowercase().contains("two factor") {
        "This account uses two-factor authentication: enter the password followed by a colon and \
         the code, like password:123456."
            .into()
    } else {
        "Ely.by refused this username or password.".into()
    }
}

/// From a username (or email) and password to an account and its secrets.
pub(super) fn sign_in(username: &str, password: &str) -> Result<(Account, Secrets), String> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|e| e.to_string())?;
    let client_token: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let (status, v) = post(
        "authenticate",
        json!({
            "username": username,
            "password": password,
            "clientToken": client_token,
            "requestUser": false,
        }),
    )?;
    if status != 200 {
        return Err(if status >= 500 {
            "Ely.by is not answering. Try again later.".into()
        } else {
            refusal(&v)
        });
    }
    let profile = v.get("selectedProfile").cloned().unwrap_or(Value::Null);
    let (Some(token), Some(uuid), Some(name)) = (
        str_field(&v, "accessToken"),
        str_field(&profile, "id"),
        str_field(&profile, "name"),
    ) else {
        return Err("This Ely.by account has no Minecraft profile.".into());
    };
    let account = Account {
        uuid: uuid.replace('-', "").to_lowercase(),
        skin: Some((format!("{SKINS}/{name}.png"), "classic".into())),
        cape: None,
        name,
    };
    let secrets = Secrets {
        refresh_token: client_token,
        sign_in_required: false,
        minecraft_token: token,
        minecraft_expires_at: now() + VALID_FOR,
        xuid: None,
        owns: true,
        elyby: true,
    };
    Ok((account, secrets))
}

/// [`sign_in`], then the account kept in the keychain and selected.
pub(super) fn sign_in_and_keep(
    username: &str,
    password: &str,
    progress: &dyn Fn(session::Phase),
) -> Result<Account, String> {
    progress(session::Phase::Elyby);
    let (account, secrets) = sign_in(username.trim(), password)?;
    progress(session::Phase::Queued);
    let store = store();
    let _operation = session::lock(store)?;
    let mut accounts = Accounts::load();
    progress(session::Phase::Saving);
    accounts.keep_sign_in(store, account.clone(), &secrets)?;
    accounts.save()?;
    Ok(account)
}

/// A token Ely.by accepts: the same one, or a renewed one. `Err` is the reason it can't be
/// told (no network) or [`SIGN_IN_AGAIN_ELYBY`].
fn valid_token(secrets: &Secrets) -> Result<String, String> {
    let token = &secrets.minecraft_token;
    let (status, _) = post("validate", json!({ "accessToken": token }))?;
    if (200..300).contains(&status) {
        return Ok(token.clone());
    }
    let (status, v) = post(
        "refresh",
        json!({
            "accessToken": token,
            "clientToken": secrets.refresh_token,
            "requestUser": false,
        }),
    )?;
    match (status, str_field(&v, "accessToken")) {
        (200, Some(renewed)) => Ok(renewed),
        (500.., _) => Err("Ely.by is not answering. Try again later.".into()),
        _ => Err(SIGN_IN_AGAIN_ELYBY.into()),
    }
}

/// [`check`](super::check) for an Ely.by account: there is no ownership to ask about, only
/// whether its token still works.
pub(super) fn check(account: &Account, mut secrets: Secrets) -> Check {
    if secrets.sign_in_required {
        return Check::NoAnswer(SIGN_IN_AGAIN_ELYBY.into(), secrets);
    }
    match valid_token(&secrets) {
        Ok(token) => {
            secrets.minecraft_token = token;
            secrets.minecraft_expires_at = now() + VALID_FOR;
            secrets.owns = true;
            Check::Owns(account.clone(), secrets)
        }
        Err(e) => {
            secrets.sign_in_required = e == SIGN_IN_AGAIN_ELYBY;
            Check::NoAnswer(e, secrets)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_factor_refusal_says_how_to_enter_the_code() {
        let v = json!({"error": "ForbiddenOperationException",
            "errorMessage": "Account protected with two factor auth."});
        assert!(refusal(&v).contains("password:123456"));
        assert!(!refusal(&json!({"errorMessage": "Invalid credentials."})).contains("two-factor"));
    }
}
