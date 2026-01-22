use anyhow::anyhow;
use esp_idf_svc::hal::sys;
use esp_idf_svc::sys::*;
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;

pub fn init_fs() -> anyhow::Result<()> {
    unsafe {
        let ret = nvs_flash_init();
        if ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND {
            // 如果 nvs 需要擦除
            nvs_flash_erase();
            nvs_flash_init();
        } else {
            esp!(ret)?;
        }
    }

    // 启用磨损均衡功能
    let mut wl_handle = 0;
    let mount_config = esp_vfs_fat_mount_config_t {
        max_files: 5,
        format_if_mount_failed: true,
        allocation_unit_size: 4096,

        disk_status_check_enable: false,
        use_one_fat: false,
    };

    // 挂载 FAT 到 /fat（分区 label 必须与 partitions.csv 中一致.
    let mount_point = String::from("/fat");
    let partition_label = String::from("storage");
    let res = unsafe {
        // 和c交互只能使用CString.
        esp_vfs_fat_spiflash_mount(
            CString::new(mount_point)?.as_ptr(),
            CString::new(partition_label)?.as_ptr(),
            &mount_config,
            &mut wl_handle as *mut wl_handle_t,
        )
    };

    if res != sys::ESP_OK {
        return Err(anyhow!(res));
    }
    test_fs_rw()?;
    Ok(())
}

/// `test_fs_rw` 测试文件系统读写
pub fn test_fs_rw() -> anyhow::Result<()> {
    let path = "/fat/hello.txt";
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .expect("create file failed");
        f.write_all(b"hello from rust on esp32!\n")?;
    }
    let mut s = String::new();
    let mut f = File::open(path)?;
    f.read_to_string(&mut s)?;
    Ok(())
}
