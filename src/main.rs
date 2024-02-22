//! Código original corresponde al ejemplo de 
//! MQTT blocking client example which subscribes to an internet MQTT server and then sends
//! and receives events in its own topic.
//! Disponible en https://github.com/esp-rs/esp-idf-svc/blob/master/examples/mqtt_client.rs 

//! Ejemplo de comando de intensidad de brillo de LEDs y encendido/apagado mediante
//! indicaciones recibidas por MQTT usando mosquito test server mqtt://test.mosquitto.org/

use core::time::Duration;

// Dependencias para el cliente MQTT
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::ledc::LedcDriver;
use esp_idf_svc::hal::ledc::*;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::*;
use esp_idf_svc::hal::units::*;
use esp_idf_svc::hal::gpio::*;

use log::*;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

const MQTT_URL: &str = "mqtt://test.mosquitto.org:1883";
const MQTT_CLIENT_ID: &str = "esp-mqtt-esp32";
const MQTT_TOPIC: &str = "esp-mqtt-leds-remoto";

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    // Configuración de los LEDs
    let peripherals = Peripherals::take().unwrap();


    let mut channel_rojo = LedcDriver::new(
        peripherals.ledc.channel1,
        LedcTimerDriver::new(
            peripherals.ledc.timer1,
            &esp_idf_svc::hal::ledc::config::TimerConfig::new().frequency(5000_u32.Hz().into()),
        ).unwrap(),
        peripherals.pins.gpio25,
    ).unwrap();

    let mut channel_verde = LedcDriver::new(
        peripherals.ledc.channel2,
        LedcTimerDriver::new(
            peripherals.ledc.timer2,
            &esp_idf_svc::hal::ledc::config::TimerConfig::new().frequency(5000_u32.Hz().into()),
        ).unwrap(),
        peripherals.pins.gpio26,
    ).unwrap();

    let mut channel_azul = LedcDriver::new(
        peripherals.ledc.channel3,
        LedcTimerDriver::new(
            peripherals.ledc.timer3,
            &esp_idf_svc::hal::ledc::config::TimerConfig::new().frequency(5000_u32.Hz().into()),
        ).unwrap(),
        peripherals.pins.gpio27,
    ).unwrap();

    let mut led_blanco = PinDriver::output(peripherals.pins.gpio18).unwrap();
    

    // Rutina de conexión WiFi
    let mut esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs.clone())).unwrap();
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop.clone()).unwrap();

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    })).unwrap();

    wifi.start().unwrap();
    info!("Wifi iniciada");

    wifi.connect().unwrap();
    info!("Wifi conectada");

    wifi.wait_netif_up().unwrap();
    info!("Wifi netif funcionando");

    let (mut client, mut conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID).unwrap();

    // run(&mut client, &mut conn,   MQTT_TOPIC).unwrap();
    run(&mut client, &mut conn,  &mut channel_rojo, &mut channel_verde, &mut channel_azul, &mut led_blanco, MQTT_TOPIC).unwrap();
}

fn run(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    led_rojo: &mut LedcDriver<'_>, 
    led_verde: &mut LedcDriver<'_>, 
    led_azul: &mut LedcDriver<'_>,
    led_blanco: &mut PinDriver<'_, Gpio18, Output>,
    topic: &str,
) -> Result<(), EspError> {
    std::thread::scope(|s| {
        info!("About to start the MQTT client");

        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                info!("MQTT esperando Mensajes!!");

                while let Ok(event) = connection.next() {
                    match event.payload() {
                        EventPayload::Received { id: _, topic: _, data, details: _ } => {
                            match data.len() {
                                1 => {
                                    match data[0] {
                                        // R o r en ASCII
                                        82 | 114 => {
                                            brightness_control(led_rojo);
                                        },
                                        // G o g en ASCII
                                        71 | 103 => {
                                            brightness_control(led_verde);
                                        },
                                        // B o b en ASCII
                                        66 | 98 => {
                                            brightness_control(led_azul);
                                        },
                                        87 | 119 => {
                                            led_blanco.toggle().unwrap();
                                        },
                                        _ => {}
                                    }                                    
                                },
                                _ => {}

                            }
                        },
                        _ => {}
                    }
                }

                info!("Connection closed");
            })
            .unwrap();

        client.subscribe(topic, QoS::AtMostOnce)?;

        info!("Subscribed to topic \"{topic}\"");


        loop {

            let sleep_secs = 2;

            info!("Esperando por {sleep_secs}s...");
            std::thread::sleep(Duration::from_secs(sleep_secs));
        }
    })
}

fn mqtt_create(
    url: &str,
    client_id: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        },
    )?;

    Ok((mqtt_client, mqtt_conn))
}


fn brightness_control(led: &mut LedcDriver<'_>) {

    let actual_brightness = led.get_duty();
    let increment = 5;

    info!("Brillo {actual_brightness}");

    if led.get_max_duty() > (actual_brightness + increment) {
        led.set_duty(actual_brightness + increment).unwrap();
    } else {
        led.set_duty(0).unwrap();
    }
}