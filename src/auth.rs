use rocket::{
    form::{Form, Strict},
    http::{Cookie, CookieJar, Status},
    outcome::{IntoOutcome, Outcome},
    response::Redirect,
    serde::json,
    State,
};
use shelby_backend::{
    database::{PrimaryKey, Record},
    user::User,
};

use super::{Config, Error};

/// Credentials of a user send for login,
#[derive(Debug, Clone, FromForm)]
pub struct Credentials {
    pub user: String,
    pub password: String,
}

impl Credentials {
    /// Check if the credentials match an existing user record.
    fn matches(&self, record: &Record<User>) -> bool {
        &record.username == &self.user && record.password_hash.matches(&self.user, &self.password)
    }
}

/// The strategy how to proced in cases of missing authorization.
pub trait Strategy: Default {
    /// Convert to object to an appropiated outcome
    fn to_outcome(
        value: Option<AuthenticatedUser<Self>>,
    ) -> Outcome<AuthenticatedUser<Self>, (Status, ()), Status>;
}

/// Forward to the next possible route or return 'Unauthorized'.
#[derive(Default)]
pub struct Forward;

impl Strategy for Forward {
    fn to_outcome(
        value: Option<AuthenticatedUser<Self>>,
    ) -> Outcome<AuthenticatedUser<Self>, (Status, ()), Status> {
        value.or_forward(Status::Unauthorized)
    }
}

/// Fail fast and return 'Unauthorized'.
#[derive(Default)]
pub struct Fail;

impl Strategy for Fail {
    fn to_outcome(
        value: Option<AuthenticatedUser<Self>>,
    ) -> Outcome<AuthenticatedUser<Self>, (Status, ()), Status> {
        value.or_error((Status::Unauthorized, ()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AuthenticatedUser<T = Fail> {
    user: PrimaryKey<User>,
    strategy: T,
}

impl<T: Strategy> AuthenticatedUser<T> {
    /// The name of the cookie used to store the ID
    pub const AUTH_COOKIE_NAME: &'static str = "shelby_auth";

    /// Login the given user.
    pub fn login(cookies: &CookieJar, user: &Record<User>) {
        cookies.add_private(
            Cookie::build((
                Self::AUTH_COOKIE_NAME,
                rocket::serde::json::to_string(&user.identifier).expect("valid serialized element"),
            ))
            .same_site(rocket::http::SameSite::Lax),
        );
    }

    /// Logout any registered user.
    pub fn logout(cookies: &CookieJar) {
        cookies.remove(Self::AUTH_COOKIE_NAME);
    }
}

#[rocket::async_trait]
impl<'r, T: Strategy> rocket::request::FromRequest<'r> for AuthenticatedUser<T> {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> Outcome<Self, (Status, Self::Error), Status> {
        T::to_outcome(
            request
                .cookies()
                .get_private(Self::AUTH_COOKIE_NAME)
                .and_then(|cookie| json::from_str(cookie.value()).ok())
                .map(|primary_key| AuthenticatedUser {
                    user: primary_key,
                    strategy: T::default(),
                }),
        )
    }
}

#[post("/users/login", data = "<credentials>")]
pub fn login(
    state: &State<Config>,
    credentials: Form<Strict<Credentials>>,
    cookies: &CookieJar,
) -> Result<Redirect, Error> {
    match User::select_by_name(&state.database(), &credentials.user) {
        Ok(Some(user)) if credentials.matches(&user) => {
            AuthenticatedUser::<Fail>::login(cookies, &user);
            Ok(Redirect::to(uri!("/")))
        }
        Ok(Some(_)) => {
            // Wrong password!
            Err(Error::WrongPassword)
        }
        Ok(None) => Err(Error::NotFound),
        Err(err) => Err(err.into()),
    }
}

#[get("/users/logout")]
pub fn logout(cookies: &CookieJar) -> Redirect {
    AuthenticatedUser::<Forward>::logout(cookies);
    Redirect::to(uri!("/"))
}
