//! Settings → Account: signing in with an Ely.by account (username or email and password), under
//! the Microsoft sign-in. The sign-in itself is `account/elyby.rs`; the window shows its progress
//! and errors where it shows Microsoft's.

use super::*;
use horizon_launcher::account::session::Phase;

impl Launcher {
    pub(super) fn elyby_section(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let busy = matches!(
            self.sign_in_state,
            Some(SignInState::Starting | SignInState::Code(_) | SignInState::Working(_))
        );
        if !self.elyby_open {
            return div().flex().mt(px(12.)).child(
                button("elyby-open", "Sign in with Ely.by", Kind::Secondary, &t).on_click(
                    cx.listener(move |this, _: &ClickEvent, window, cx| {
                        if !busy {
                            this.elyby_open = true;
                            window.focus(&this.elyby_user_focus, cx);
                        }
                        cx.notify();
                    }),
                ),
            );
        }
        let user = self.elyby_field(false, cx);
        let pass = self.elyby_field(true, cx);
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .mt(px(12.))
            .child(user)
            .child(pass)
            .child(div().text_color(t.ink_soft).text_size(px(12.)).child(tr(
                "Ely.by accounts play without a Mojang purchase. With two-factor sign-in on, \
                 type the password, a colon and the code: password:123456.",
            )))
            .child(
                div()
                    .flex()
                    .gap(px(4.))
                    .child(
                        button("elyby-submit", "Sign in", Kind::Primary, &t).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.submit_elyby();
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        button("elyby-cancel", "Cancel", Kind::Ghost, &t).on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                this.close_elyby();
                                cx.notify();
                            },
                        )),
                    ),
            )
    }

    /// One of the two fields: the username (Enter goes on to the password) or the password
    /// (dots; Enter signs in).
    fn elyby_field(&self, password: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let (focus, focused, placeholder) = if password {
            (&self.elyby_pass_focus, self.elyby_pass_focused, "Password")
        } else {
            (
                &self.elyby_user_focus,
                self.elyby_user_focused,
                "Ely.by username or email",
            )
        };
        let edit = if password {
            &self.elyby_pass
        } else {
            &self.elyby_user
        };
        let field = div()
            .id(if password { "elyby-pass" } else { "elyby-user" })
            .track_focus(focus)
            .flex()
            .items_center()
            .w(px(300.))
            .h(px(30.))
            .px(px(12.))
            .overflow_hidden()
            .border_2()
            .border_color(if focused { t.accent } else { t.outline })
            .bg(t.well)
            .text_size(px(13.))
            .cursor_text()
            .child(line_edit::content(
                edit, &t, focused, false, password, placeholder,
            ))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                window.focus(this.elyby_focus(password), cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                match line_edit::handle(this.elyby_edit(password), ev, cx) {
                    line_edit::Key::Pass => return,
                    line_edit::Key::Escape => window.blur(cx),
                    line_edit::Key::Enter if password => this.submit_elyby(),
                    line_edit::Key::Enter => window.focus(&this.elyby_pass_focus, cx),
                    _ => {}
                }
                cx.stop_propagation();
                cx.notify();
            }));
        if password {
            line_edit::mouse(field, cx, |this| &mut this.elyby_pass)
        } else {
            line_edit::mouse(field, cx, |this| &mut this.elyby_user)
        }
    }

    fn elyby_focus(&self, password: bool) -> &FocusHandle {
        if password {
            &self.elyby_pass_focus
        } else {
            &self.elyby_user_focus
        }
    }

    fn elyby_edit(&mut self, password: bool) -> &mut line_edit::LineEdit {
        if password {
            &mut self.elyby_pass
        } else {
            &mut self.elyby_user
        }
    }

    fn submit_elyby(&mut self) {
        let user = self.elyby_user.text().trim().to_string();
        if user.is_empty() || self.elyby_pass.text().is_empty() {
            return;
        }
        // The password leaves the field the moment it is sent.
        let pass = self.elyby_pass.take();
        self.cancel_sign_in();
        self.sign_in = Some(SignIn::start_elyby(user, pass));
        self.sign_in_state = Some(SignInState::Working(Phase::Elyby));
        self.elyby_open = false;
    }

    fn close_elyby(&mut self) {
        self.elyby_open = false;
        self.elyby_pass.take();
    }
}
