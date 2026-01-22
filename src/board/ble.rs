use crate::board::peripherals::BoardEsp32State;
use esp32_nimble::{uuid128, BLEAdvertisementData, BLEDevice, NimbleProperties};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub fn ble_server_start(
    board: Arc<Mutex<BoardEsp32State>>,
) -> anyhow::Result<JoinHandle<anyhow::Result<()>>, anyhow::Error> {
    let ble = BLEDevice::take();
    let ble_advertising = ble.get_advertising();
    let server = ble.get_server();
    server.on_connect(|server, desc| {
        log::info!("Client connected: {desc:?}");

        // 优化通信, 低功耗使用
        server
            .update_conn_params(desc.conn_handle(), 24, 48, 0, 60)
            .unwrap();

        // 没达到最大连接设备数就继续广播
        if server.connected_count() < (esp_idf_svc::sys::CONFIG_BT_NIMBLE_MAX_CONNECTIONS as _) {
            log::info!("Multi-connect support: start advertising");
            ble_advertising.lock().start().unwrap();
        }
    });

    server.on_disconnect(|_desc, reason| {
        log::info!("Disconnected from server: {reason:?}");
    });
    let service = server.create_service(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa"));
    let static_characteristic = service.lock().create_characteristic(
        uuid128!("d4e0e0d0-1a2b-11e9-ab14-d663bd873d93"),
        NimbleProperties::READ,
    );
    static_characteristic
        .lock()
        .set_value(b"Hello World, this is static, TOTHTOT");

    // 通知特征, 能够向订阅这个uuid的设备不停发送消息
    let notifying_characteristic = service.lock().create_characteristic(
        uuid128!("a3c87500-8ed3-4bdf-8a39-a01bebede295"),
        NimbleProperties::READ | NimbleProperties::NOTIFY,
    );
    notifying_characteristic
        .lock()
        .set_value(b"Hello World, this is notify, TOTHTOT");

    // 写入特征, 通过这个uuid能够向esp发送数据
    let write_characteristic = service.lock().create_characteristic(
        uuid128!("3c9a3f00-8ed3-4bdf-8a39-a01bebede295"),
        NimbleProperties::READ | NimbleProperties::WRITE,
    );
    write_characteristic
        .lock()
        .on_read(move |characteristic, desc| {
            log::info!("characteristic: {characteristic:?}, {desc:?}");
        })
        .on_write(|args| {
            log::info!(
                "wrote to write_characteristic: {:?} {:?}",
                args.current_data(),
                args.recv_data()
            );
        });

    // 设置蓝牙名称, 以及透传uuid, 开始蓝牙服务
    ble_advertising.lock().set_data(
        BLEAdvertisementData::new()
            .name("ESP32-GATT-Server")
            .add_service_uuid(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa")),
    )?;
    ble_advertising.lock().start()?;

    // 开启连接日志显示
    server.ble_gatts_show_local();

    let handle = thread::spawn(move || -> anyhow::Result<()> {
        let mut counter = 0;
        loop {
            thread::sleep(Duration::from_millis(1000));
            let board_state = board.lock().expect("Failed to lock board mutex");
            let temp = board_state.current_mcu_temperature;
            if board_state.exit {
                log::info!("ble server stopped");
                break Ok(());
            }
            let notify_str = format!("running:{counter},temp:{temp}",);
            // log::info!("{notify_str}");
            notifying_characteristic
                .lock()
                .set_value(notify_str.as_bytes())
                .notify();
            counter += 1;
        }
    });
    Ok(handle)
}
