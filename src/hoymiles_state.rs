use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sha2::{Digest, Sha256};
use tracing::{error, info};

pub struct HoymilesState {
    pub token: Option<String>,
    pub sid: Option<String>,
}

impl HoymilesState {
    fn new() -> Self {
        Self {
            token: None,
            sid: None,
        }
    }
}

pub async fn init(client: &reqwest::Client) -> HoymilesState {
    let mut state = HoymilesState::new();
    if state.token.is_none() {
        info!("No token found. Requesting a new one.");
        state.token = request_new_token(client).await;
        if state.token.is_some() {
            info!("Successfully obtained new token");
        } else {
            error!("Failed to obtain token");
        }
    }
    if let Some(token) = &state.token {
        if state.sid.is_none() {
            info!("No sid found. Requesting a new one.");
            state.sid = get_sid(client, token).await;
            if state.sid.is_some() {
                info!("Successfully obtained SID");
            } else {
                error!("Failed to obtain SID");
            }
        }
    }

    state
}

async fn request_new_token(client: &reqwest::Client) -> Option<String> {
    let username = match std::env::var("HOYMILES_USERNAME") {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to get HOYMILES_USERNAME: {}", e);
            return None;
        }
    };
    let password = match std::env::var("HOYMILES_PASSWORD") {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to get HOYMILES_PASSWORD: {}", e);
            return None;
        }
    };

    // Get region
    let region_url = "https://euapi.hoymiles.com/iam/pub/0/c/region_c";
    let region_resp = match client
        .post(region_url)
        .json(&serde_json::json!({ "email": username }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get region: {}", e);
            return None;
        }
    };

    let region_data: serde_json::Value = match region_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse region response: {}", e);
            return None;
        }
    };

    let login_url = match region_data["data"]["login_url"].as_str() {
        Some(url) => url,
        None => {
            error!("No login URL in region response");
            return None;
        }
    };

    // Generate password hash
    let md5_hash = format!("{:x}", md5::compute(password.as_bytes()));
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let sha256_hash = BASE64.encode(hasher.finalize());
    let encoded_password = format!("{}.{}", md5_hash, sha256_hash);

    // Login
    let login_url = format!("{}/iam/pub/0/c/login_c", login_url);
    let login_resp = match client
        .post(login_url)
        .json(&serde_json::json!({
            "user_name": username,
            "password": encoded_password
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Login request failed: {}", e);
            return None;
        }
    };

    let login_data: serde_json::Value = match login_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse login response: {}", e);
            return None;
        }
    };

    match login_data["data"]["token"].as_str() {
        Some(token) => Some(token.to_string()),
        None => {
            error!("No token in login response");
            None
        }
    }
}

async fn get_sid(client: &reqwest::Client, token: &str) -> Option<String> {
    let url = "https://neapi.hoymiles.com/pvmc/api/0/station/select_by_page_c";
    let resp = match client
        .post(url)
        .header("Authorization", token)
        .json(&serde_json::json!({
            "page": 1,
            "page_size": 50
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to get SID: {}", e);
            return None;
        }
    };

    let data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse SID response: {}", e);
            return None;
        }
    };

    match &data["data"]["list"][0]["sid"].as_number() {
        Some(sid) => Some(sid.to_string()),
        None => {
            error!("No SID in response");
            None
        }
    }
}
