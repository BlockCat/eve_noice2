// use rocket::response::Redirect;
// use rocket_oauth2::{OAuth2, TokenResponse};

// #[get("/login/eve")]
// fn eve_login(oauth2: OAuth2<GitHub>, mut cookies: Cookies<'_>) -> Redirect {
//     // We want the "user:read" scope. For some providers, scopes may be
//     // pre-selected or restricted during application registration. We could
//     // use `&[]` instead to not request any scopes, but usually scopes
//     // should be requested during registation, in the redirect, or both.
//     oauth2.get_redirect(&mut cookies, &["user:read"]).unwrap()
// }

// // This route, mounted at the application's Redirect URI, uses the
// // `TokenResponse` request guard to complete the token exchange and obtain
// // the token.
// //
// // The order is important here! If Cookies is positioned before
// // TokenResponse, TokenResponse will be unable to verify the token exchange
// // and return a failure.
// #[get("/auth/eve")]
// fn eve_callback(token: TokenResponse<GitHub>, mut cookies: Cookies<'_>) -> Redirect {
//     // Set a private cookie with the access token
//     cookies.add_private(
//         Cookie::build("token", token.access_token().to_string())
//             .same_site(SameSite::Lax)
//             .finish(),
//     );
//     Redirect::to("/")
// }
