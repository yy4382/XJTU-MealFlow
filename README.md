# XJTU MealFlow

你在西交吃了啥？

## 快速开始

### 获取 Account 和 Cookie

在[校园卡网站](https://card.xjtu.edu.cn)（目前学校运营商网络无法访问，可以使用流量或者校园网访问）获取account和hallticket。

TODO

### 运行

从 Release 下载对应系统的二进制文件，打开终端, 设置环境变量 `XMF_ACCOUNT` 和 `XMF_COOKIE`，然后运行。

```bash
# macOS, Linux
export XMF_ACCOUNT=your_account
export XMF_COOKIE="hallticket=your_hallticket"
./xjtu-mealflow
```

```powershell
# Windows
$env:XMF_ACCOUNT="your_account"
$env:XMF_COOKIE="hallticket=your_hallticket"
.\xjtu-mealflow.exe
```

## License

Copyright (c) Chris Yang <yy4382@outlook.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
