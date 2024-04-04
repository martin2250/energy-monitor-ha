use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use esp_println::println;
use picoserve::{
    response::{IntoResponse, Json, StatusCode},
    routing::{get, post},
    Config, Timeouts,
};
use static_cell::make_static;

use crate::{
    config::CONFIG_SERVER_ENABLE,
    wifi::{StackAp, StackSta},
};

use super::{
    json_body::JsonBody, save_config, CalibrationConfig, MqttConfig, StpmConfig, WifiConfig, CONFIG_CALIBRATION, CONFIG_MQTT, CONFIG_STPM, CONFIG_WIFI
};

const KEEP_PASSWORD: &str = "--keep--";

type AppRouter = impl picoserve::routing::PathRouter;

const SERVER_CONFIG: Config<Duration> = Config::new(Timeouts {
    start_read_request: Some(Duration::from_secs(5)),
    read_request: Some(Duration::from_secs(1)),
    write: Some(Duration::from_secs(1)),
});

#[derive(Default)]
pub struct ServerState {
    pub mqtt: MqttConfig,
    pub wifi: WifiConfig,
    pub stpm: StpmConfig,
    pub calibration: CalibrationConfig,
}

pub static STATE: Mutex<CriticalSectionRawMutex, Option<ServerState>> = Mutex::new(None);

#[embassy_executor::task]
pub async fn run_config_server(stack_sta: &'static StackSta, stack_ap: Option<&'static StackAp>) {
    STATE.lock().await.replace(Default::default());

    fn make_app() -> picoserve::Router<AppRouter> {
        picoserve::Router::new()
            .route("/", get(|| async move { "Hello World" }))
            .route(
                "/config_mqtt.json",
                get(get_config_mqtt).post(post_config_mqtt),
            )
            .route(
                "/config_wifi.json",
                get(get_config_wifi).post(post_config_wifi),
            )
            .route(
                "/config_calibration.json",
                get(get_config_calibration).post(post_config_calibration),
            )
            .route(
                "/config_stpm.json",
                get(get_config_stpm).post(post_config_stpm),
            )
            .route("/save", post(post_save_config))
    }

    let app = make_static!(make_app());
    let port = 80;
    let mut enable_server = CONFIG_SERVER_ENABLE.wait().await;

    loop {
        if !enable_server {
            println!("config server disabled");
            enable_server = CONFIG_SERVER_ENABLE.wait().await;
            continue;
        }
        println!("config server enabled");

        if let Some(stack_ap) = stack_ap {
            let mut tcp_rx_buffer = [0; 1024];
            let mut tcp_tx_buffer = [0; 1024];
            let mut http_buffer = [0; 2048];

            let fut_sta = picoserve::listen_and_serve(
                0,
                app,
                &SERVER_CONFIG,
                stack_sta,
                port,
                &mut tcp_rx_buffer,
                &mut tcp_tx_buffer,
                &mut http_buffer,
            );

            let mut tcp_rx_buffer = [0; 1024];
            let mut tcp_tx_buffer = [0; 1024];
            let mut http_buffer = [0; 2048];

            let fut_ap = picoserve::listen_and_serve(
                0,
                app,
                &SERVER_CONFIG,
                stack_ap,
                port,
                &mut tcp_rx_buffer,
                &mut tcp_tx_buffer,
                &mut http_buffer,
            );

            if let Either3::First(new_enable) =
                select3(CONFIG_SERVER_ENABLE.wait(), fut_sta, fut_ap).await
            {
                enable_server = new_enable;
                continue;
            };
        } else {
            let mut tcp_rx_buffer = [0; 1024];
            let mut tcp_tx_buffer = [0; 1024];
            let mut http_buffer = [0; 2048];

            let fut_sta = picoserve::listen_and_serve(
                0,
                app,
                &SERVER_CONFIG,
                stack_sta,
                port,
                &mut tcp_rx_buffer,
                &mut tcp_tx_buffer,
                &mut http_buffer,
            );

            if let Either::First(new_enable) = select(CONFIG_SERVER_ENABLE.wait(), fut_sta).await {
                enable_server = new_enable;
                continue;
            };
        }
    }
}

// -----------------------------------------------------------------------------

async fn get_config_mqtt() -> impl IntoResponse {
    let state = STATE.lock().await;
    let state = state.as_ref().unwrap();

    let mut config = state.mqtt.clone();
    config.mqtt_password = KEEP_PASSWORD.try_into().unwrap();
    Json(config)
}

async fn post_config_mqtt(JsonBody(mut new_config): JsonBody<MqttConfig>) -> impl IntoResponse {
    if !new_config.validate() {
        return (StatusCode::BAD_REQUEST, "config validation falid");
    }

    let mut state = STATE.lock().await;
    let state = state.as_mut().unwrap();

    if new_config.mqtt_password == KEEP_PASSWORD {
        new_config.mqtt_password = state.mqtt.mqtt_password.clone();
    }

    state.mqtt = new_config.clone();
    CONFIG_MQTT.signal(new_config);

    (StatusCode::OK, "OK")
}

// -----------------------------------------------------------------------------

async fn get_config_wifi() -> impl IntoResponse {
    let state = STATE.lock().await;
    let state = state.as_ref().unwrap();

    let mut config = state.wifi.clone();
    config.wifi_key = KEEP_PASSWORD.try_into().unwrap();
    Json(config)
}

async fn post_config_wifi(JsonBody(mut new_config): JsonBody<WifiConfig>) -> impl IntoResponse {
    let mut state = STATE.lock().await;
    let state = state.as_mut().unwrap();

    if new_config.wifi_key == KEEP_PASSWORD {
        new_config.wifi_key = state.wifi.wifi_key.clone();
    }

    state.wifi = new_config.clone();
    CONFIG_WIFI.signal(new_config);
}

// -----------------------------------------------------------------------------

async fn get_config_calibration() -> impl IntoResponse {
    let state = STATE.lock().await;
    let state = state.as_ref().unwrap();

    Json(state.calibration.clone())
}

async fn post_config_calibration(
    JsonBody(new_config): JsonBody<CalibrationConfig>,
) -> impl IntoResponse {
    let mut state = STATE.lock().await;
    let state = state.as_mut().unwrap();

    state.calibration = new_config.clone();
    CONFIG_CALIBRATION.signal(new_config);
}

// -----------------------------------------------------------------------------

async fn get_config_stpm() -> impl IntoResponse {
    let state = STATE.lock().await;
    let state = state.as_ref().unwrap();

    Json(state.stpm.clone())
}

async fn post_config_stpm(JsonBody(new_config): JsonBody<StpmConfig>) -> impl IntoResponse {
    let mut state = STATE.lock().await;
    let state = state.as_mut().unwrap();

    state.stpm = new_config.clone();
    CONFIG_STPM.signal(new_config);
}

// -----------------------------------------------------------------------------

async fn post_save_config() -> impl IntoResponse {
    if save_config().await == Some(()) {
        (StatusCode::OK, "OK")
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "error while saving to flash")
    }
}
