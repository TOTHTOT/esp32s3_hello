# WSL配置rust开发esp32s3

## 环境配置

- 参考[Rust + ESP IDF 开发环境配置](https://www.yuque.com/haobogu/vgcc41/wlhc4qy3tisfmqph)链接配置基本功能, 配置到第五步即可 .
  
  - 安装espup 使用`cargo install espup --locked`, 确保不会因为环境问题导致编译失败.
  
  - 安装完成后每次重启都要执行`. /root/export-esp.sh`重新配置环境.

- 安装`cargo install cargo-generate`用于生成代码.
  
  - `cargo generate esp-rs/esp-template`或者`cargo generate esp-rs/esp-idf-template cargo`命令创建项目

- 网络配置
  
  - 使用`Clash for Windows`开启允许局域网.
  
  - 使用`cat /etc/resolv.conf|grep nameserver|awk '{print $2}'`查看宿主机ip预期得到`172.30.64.1`
  
  - 将ip和端口分别设置为宿主机ip和`Clash for Windows`的端口, 并输入到wsl的终端, 使用`wget www.google.com`测试是否连接成功.
  
  ```shell
  export http_proxy=http://172.24.80.1:1164
  export https_proxy=http://172.24.80.1:1164
  export all_proxy=http://172.24.80.1:1164
  ```
  
  - 在`.bashrc`文件内添加如下内容,设置网络代理`unset http_proxy https_proxy all_proxy HTTP_PROXY HTTPS_PROXY`命令取消配置.
  
  ```shell
  # 终端启动时自动配置代理（IP 从 /etc/resolv.conf 自动获取）
  function auto_set_proxy() {
      # 1. 从 /etc/resolv.conf 中提取第一个 nameserver（通常是 WSL2 网关 IP）
      # 用 grep 过滤 nameserver 行，awk 提取第二列（IP），head -n 1 取第一个结果
      local proxy_ip=$(grep -m 1 '^nameserver' /etc/resolv.conf | awk '{print $2}')
      
      # 2. 检查是否成功获取到 IP（避免空值）
      if [ -z "$proxy_ip" ]; then
          echo "⚠️  自动获取代理 IP 失败，请检查 /etc/resolv.conf"
          return 1
      fi
      
      # 3. 配置代理环境变量（端口固定为你的 1164，若端口会变可改为变量）
      local proxy_port=1164
      export http_proxy="http://${proxy_ip}:${proxy_port}"
      export https_proxy="http://${proxy_ip}:${proxy_port}"
      export all_proxy="http://${proxy_ip}:${proxy_port}"
      # 兼容部分工具的大写变量
      export HTTP_PROXY="$http_proxy"
      export HTTPS_PROXY="$https_proxy"
      
      # 4. 提示代理配置成功（可选，方便确认）
      echo "✅ 自动配置代理成功：$http_proxy"
  }
  
  # 终端启动时执行函数（关键：让自动配置生效）
  auto_set_proxy
  ```
  
  - wsl下进行GitHub代码提交, 
  
  ```shell
  ssh-keygen -t ed25519 -C "mczyfs@gmail.com" # 生成key, 如果没有的话
  cat ~/.ssh/id_ed25519.pub # 复制key到 登录 GitHub → 点击头像 → Settings → SSH and GPG keys → New SSH key；
  # 在项目目录下
  git remote remove origin
  git remote add origin git@github.com:TOTHTOT/esp32_hello.git
  ssh -T git@github.com # 测试链接, 正常返回:Hi TOTHTOT! You've successfully authenticated...
  git push -u origin main # 推送代码
  
  
  # 如果网络不通就在 ~/.ssh/config 文件内追加如下内容, 不存在就创建
  Host github.com
    Hostname ssh.github.com
    Port 443
    User git
  ```

- 映射串口
  
  - 安装`[Releases · dorssel/usbipd-win](https://github.com/dorssel/usbipd-win/releases)`
  - 安装[GitHub - nickbeth/wsl-usb-manager: A fast and light GUI for usbipd-win. Manage connecting USB devices to WSL.](https://github.com/nickbeth/wsl-usb-manager)绑定设备到wsl.
  
  ```shell
  root@DESKTOP-PH29EBJ:/mnt/c/WINDOWS/system32# lsusb
  root@DESKTOP-PH29EBJ:/mnt/c/WINDOWS/system32# lsusb
  Bus 002 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
  Bus 001 Device 002: ID 303a:4001 Espressif Systems Espressif Device
  Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
  ```

- 参考[使用WSL2+Rust+RustRover+esp-rs进行ESP32嵌入式开发 :: 世界观察日志](https://wol.moe/%E4%BD%BF%E7%94%A8wsl2-rust-rustrover-esp-rs%E8%BF%9B%E8%A1%8Cesp32%E5%B5%8C%E5%85%A5%E5%BC%8F%E5%BC%80%E5%8F%91/)部署到RustRover编译.

- idf配置, 手动配置`sdkconfig.defaults`再复制到项目, 比较麻烦
  - 拉取仓库
    ```shell
    git clone --recursive https://github.com/espressif/esp-idf.git # 递归拉取仓库
    git config --global core.autocrlf input # 设置拉取的格式换行符
    # 如果不行就手动替换换行符
    find . -name "*.sh" -exec dos2unix {} \;
    find . -name "*.py" -exec dos2unix {} \;
    
    ./install.sh
    source export.sh 
    # 之后就可以在项目根目录配置idf了.
   ```

## 新建工程

- 创建流程, 不能选错芯片`cargo generate esp-rs/esp-idf-template cargo`和这个类似.

```shell
root@DESKTOP-PH29EBJ:~# cargo generate esp-rs/esp-template
⚠️   Favorite `esp-rs/esp-template` not found in config, using it as a git repository: https://github.com/esp-rs/esp-template.git

============================================
⚠️ This template is no longer supported. ⚠️
============================================

esp-template is no longer supported and we do not recommend using it.
We suggest using esp-generate instead to get you started.
For more information about esp-generate, visit https://github.com/esp-rs/esp-generate

🤷   Project Name: esp32s3_hello
🔧   Destination: /root/esp32s3_hello ...
🔧   project-name: esp32s3_hello ...
🔧   Generating template ...
✔ 🤷   Which MCU to target? · esp32s3
✔ 🤷   Configure advanced template options? · false
[ 1/15]   Done: .cargo/config.toml                                                                                      [ 2/15]   Done: .cargo                                                                                                  [ 3/15]   Done: .gitignore                                                                                              [ 4/15]   Done: .vscode/settings.json                                                                                   [ 5/15]   Done: .vscode                                                                                                 [ 6/15]   Done: Cargo.toml                                                                                              [ 7/15]   Done: LICENSE-APACHE                                                                                          [ 8/15]   Done: LICENSE-MIT                                                                                             [ 9/15]   Done: build.rs                                                                                                [10/15]   Ignored: init-script.rhai                                                                                     [11/15]   Ignored: post-script.rhai                                                                                     [12/15]   Ignored: pre-script.rhai                                                                                      [13/15]   Done: rust-toolchain.toml                                                                                     [14/15]   Done: src/main.rs                                                                                             [15/15]   Done: src                                                                                                     ✔ 🤷   The template is requesting to run the following command. Do you agree?
cargo fmt · yes
🔧   Moving generated files into: `/root/esp32s3_hello`...
🔧   Initializing a fresh Git repository
✨   Done! New project created /root/esp32s3_hello
root@DESKTOP-PH29EBJ:~#
```

- 截至2025年8月25日使用`cargo generate esp-rs`创建的项目不能编译通过
  
  - 报错如下
  
  ```shell
  error[E0787]: the `asm!` macro is not allowed in naked functions
     --> /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/xtensa-lx-rt-0.17.2/src/exception/context.rs:407:5
      |
  407 | /     asm!(
  408 | |         "
  409 | |         s32e    a0,  a13, -16
  410 | |         l32e    a0,  a1,  -12
  ...   |
  425 | |         options(noreturn)
  426 | |     );
      | |_____^ consider using the `naked_asm!` macro instead
  
  error[E0787]: the `asm!` macro is not allowed in naked functions
     --> /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/xtensa-lx-rt-0.17.2/src/exception/context.rs:433:5
      |
  433 | /     asm!(
  434 | |         "
  435 | |         l32e    a0,  a13, -16
  436 | |         l32e    a1,  a13, -12
  ...   |
  451 | |         options(noreturn)
  452 | |     );
      | |_____^ consider using the `naked_asm!` macro instead
  ```

- `cargo build --release`构建

- `cargo run --release`运行

### 备注

- Windows下的部署流程大致相似, 但是会出现文件夹名称太长问题
  
  ```powershell
  PS D:\esp32s3_hello> cargo build
     Compiling bindgen v0.71.1
     Compiling embuild v0.33.1
     Compiling esp-idf-sys v0.36.1
     Compiling esp-idf-hal v0.45.2
     Compiling esp-idf-svc v0.51.0
     Compiling esp32s3_hello v0.1.0 (D:\esp32s3_hello)
  error: failed to run custom build command for `esp-idf-sys v0.36.1`
  
  Caused by:
    process didn't exit successfully: `D:\esp32s3_hello\target\debug\build\esp-idf-sys-8d83a81b53715865\build-sc
  ript-build` (exit code: 1)
    --- stderr
    Error: Too long output directory: `\\?\D:\esp32s3_hello\target\xtensa-esp32s3-espidf\debug\build\esp-idf-sys
  -05a381348ec5bec9\out`. Shorten your project path down to no more than 10 characters (or use WSL2 and its nati
  ve Linux filesystem). Note that tricks like Windows `subst` do NOT work!
  PS D:\esp32s3_hello>
  
  ```
  
  - 使用代理加速下载
  
  ```powershell
  # 设置 HTTP 代理（例如 Clash 默认端口 7890）
  $env:HTTP_PROXY = "http://127.0.0.1:7890"
  $env:HTTPS_PROXY = "http://127.0.0.1:7890"
  ```
