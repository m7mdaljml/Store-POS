use std::sync::RwLock;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

/// The authoritative signed-in identity for the running process.
///
/// Previously every command trusted the `user_id` the webview sent, and
/// authorization lived only in the Vue router / `v-can` directive. Both could
/// be bypassed from the webview. A successful `login` now records this session
/// on the Rust side; privileged commands verify it before doing any work.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub user_id: i64,
    pub role_id: i64,
    pub role_name: String,
    pub permissions: Vec<String>,
    pub must_change_password: bool,
}

impl Session {
    pub fn is_admin(&self) -> bool {
        self.role_name == "Admin"
    }
}

#[derive(Default)]
pub struct SessionState {
    inner: RwLock<Option<Session>>,
}

/// Records the active session after a successful sign-in.
pub fn set<R: Runtime>(app: &AppHandle<R>, session: Session) {
    let state = app.state::<SessionState>();
    *state.inner.write().unwrap() = Some(session);
}

/// Clears the active session (logout or account disable).
pub fn clear<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<SessionState>();
    *state.inner.write().unwrap() = None;
}

/// Returns the active session, if any, without enforcing anything.
pub fn current<R: Runtime>(app: &AppHandle<R>) -> Option<Session> {
    let state = app.state::<SessionState>();
    let session = state.inner.read().unwrap().clone();
    session
}

/// Requires an active, fully-activated session and (optionally) a permission.
///
/// `Admin` is treated as holding every permission, mirroring the frontend
/// router so authorization is consistent between the two layers. A user who is
/// still on a temporary password is blocked from every operation except
/// setting a new password.
pub fn require<R: Runtime>(app: &AppHandle<R>, permission: Option<&str>) -> Result<Session, String> {
    let session = current(app).ok_or_else(|| "You are not signed in".to_string())?;
    if session.must_change_password {
        return Err("You must set a new password before continuing".into());
    }
    if let Some(code) = permission {
        if !session.is_admin() && !session.permissions.iter().any(|p| p == code) {
            return Err(format!(
                "You do not have permission to perform this action ({code})"
            ));
        }
    }
    Ok(session)
}

/// Requires any active session, even one that still needs a password change
/// (used only by `set_own_password`).
pub fn require_session<R: Runtime>(app: &AppHandle<R>) -> Result<Session, String> {
    current(app).ok_or_else(|| "You are not signed in".to_string())
}

/// Requires an active session whose role is `Admin`.
pub fn require_admin<R: Runtime>(app: &AppHandle<R>) -> Result<Session, String> {
    let session = current(app).ok_or_else(|| "You are not signed in".to_string())?;
    if session.must_change_password {
        return Err("You must set a new password before continuing".into());
    }
    if !session.is_admin() {
        return Err("Administrator access is required".into());
    }
    Ok(session)
}

/// Clears the "must change password" flag after the user sets a permanent one.
pub fn mark_password_changed<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<SessionState>();
    if let Some(session) = state.inner.write().unwrap().as_mut() {
        session.must_change_password = false;
    };
}
