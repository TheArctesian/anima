//! The sign-in inside the app (mobile plan M3-5, MOB-043): the command of the phone shell. Only the
//! iPhone app has the page for it; on Android the command says that the build has no such sign-in.

use tauri::{State, Webview};

use super::{Cloud, CloudError, from_main};

/// How long a sign-in inside the app may take.
#[cfg(target_os = "ios")]
const SESSION_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);

/// Signs in inside the app (mobile plan M3-5): `true` when the person signed in, `Cancelled` when
/// the person closed the page. Only the iPhone app has it; on Android the build has no such sign-in.
#[tauri::command]
pub async fn cloud_sign_in_session(
    webview: Webview,
    cloud: State<'_, Cloud>,
) -> Result<bool, CloudError> {
    from_main(&webview)?;
    #[cfg(target_os = "ios")]
    return session_sign_in(&webview, &cloud).await;
    #[cfg(not(target_os = "ios"))]
    {
        let _ = cloud;
        Err(CloudError::Unconfigured)
    }
}

/// Shows the sign-in page of the account in the web authentication session of iOS, waits for the
/// browser at the loopback return address, and trades the code for tokens. The code, the verifier
/// and the tokens stay in the shell.
#[cfg(target_os = "ios")]
async fn session_sign_in(webview: &Webview, cloud: &Cloud) -> Result<bool, CloudError> {
    use tauri::Manager;
    use tauri_plugin_account_session::AccountSession;

    use super::{client_id, loopback, pkce, workos};
    use zeroize::Zeroizing;

    let id = client_id()?;
    let mut random = Zeroizing::new([0_u8; 64]);
    getrandom::fill(&mut *random).map_err(|_| CloudError::Refused)?;
    let attempt = pkce::Attempt::from_random(&random);
    let listener = loopback::listen(loopback::PORT).await?;
    let page = attempt
        .page(id, &loopback::address(loopback::PORT))
        .ok_or(CloudError::Refused)?;
    let app = webview.app_handle().clone();
    let shown = tauri::async_runtime::spawn_blocking(move || {
        app.state::<AccountSession<tauri::Wry>>().open(&page)
    });
    let code = tokio::select! {
        code = loopback::returned(&listener, &attempt.state) => code,
        // The page went away before the browser came back: the person closed it, or it never showed.
        closed = shown => match closed {
            Ok(Ok(())) => Err(CloudError::Cancelled),
            _ => Err(CloudError::Refused),
        },
        () = tokio::time::sleep(SESSION_LIMIT) => Err(CloudError::Expired),
    };
    let app = webview.app_handle().clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        app.state::<AccountSession<tauri::Wry>>().close();
    })
    .await;
    let tokens = workos::exchange(&cloud.http, id, &code?, &attempt.verifier).await?;
    cloud.keep(id, &tokens)?;
    Ok(true)
}
