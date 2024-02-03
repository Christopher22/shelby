use rocket::{form::{Form, Strict},
    http::{Cookie, CookieJar}, outcome::IntoOutcome, request::Outcome, response::Redirect, serde::json
};
use shelby_backend::{
    user::User,
    PrimaryKey, Record,
};

use super::{DatabaseState, Error};

/// Credentials of a user send for login,
#[derive(Debug, Clone, FromForm)]
pub struct Credentials {
    pub user: String,
    pub password: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticatedUser {
    user: PrimaryKey<User>,
}

impl AuthenticatedUser {
    /// The name of the cookie used to store the ID
    pub const AUTH_COOKIE_NAME: &'static str = "shelby_auth";

    /// Login the given user.
    pub fn login(cookies: &CookieJar, user: &Record<User>) {
        cookies.add_private(
            Cookie::build((
                AuthenticatedUser::AUTH_COOKIE_NAME,
                rocket::serde::json::to_string(&user.identifier).expect("valid serialized element"),
            ))
            .path("/")
            .secure(true),
        );
    }

    /// Logout any registered user.
    pub fn logout(cookies: &CookieJar) {
        cookies.remove(AuthenticatedUser::AUTH_COOKIE_NAME);
    }
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for AuthenticatedUser {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .cookies()
            .get_private(AuthenticatedUser::AUTH_COOKIE_NAME)
            .and_then(|cookie| json::from_str(cookie.value()).ok())
            .map(|primary_key| AuthenticatedUser { user: primary_key })
            .or_error((rocket::http::Status::Unauthorized, ()))
    }
}

#[post("/users/login", data = "<credentials>")]
pub fn login(
    state: &DatabaseState,
    credentials: Form<Strict<Credentials>>,
    cookies: &CookieJar,
) -> Result<Redirect, Error> {
    match User::select_by_name(&state.lock().expect("database mutex"), &credentials.user) {
        Ok(Some(user)) => {
            AuthenticatedUser::login(cookies, &user);
            Ok(Redirect::to(uri!("/")))
        }
        Ok(None) => Err(Error::NotFound),
        Err(err) => Err(err.into()),
    }
}

#[get("/users/logout")]
pub fn logout(cookies: &CookieJar) -> Redirect {
    AuthenticatedUser::logout(cookies);
    Redirect::to(uri!("/"))
}
