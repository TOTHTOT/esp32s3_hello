# esp32s3 核心板的基础功能

 - [x] 驱动`ws2812`.
 - [x] 添加wifi功能.
 - [x] 使用内部16M flash挂载fatfs, 使用自定义分区表, `./cargo/config.toml`内添加如下功能烧录分区表.
    ```toml
    [target.xtensa-esp32s3-espidf]
    linker = "ldproxy"
    runner = "espflash flash --monitor --partition-table partitions.csv"
    rustflags = [ "--cfg",  "espidf_time64"]
    ```
 - [x] 添加ble.
 - [x] 添加http服务器.
 - [x] 读取芯片内部温度传感器.