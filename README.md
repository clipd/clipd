# clipd

一个简单的剪切板服务。

## 功能

- [x] 自动移除剪切板文本开头的换行符
- [x] 自动移除剪切板文本中的 `<CR>`
- [x] 自动移除剪切板文本末尾的换行符
- [ ] 其它功能待续

## 安装

### Windows

```ps1
scoop bucket add clipd https://github.com/clipd/scoop-buckets
scoop install clipd
```

### Linux

开发中

### MacOS

开发中

## 使用

```bash
# 以无服务模式运行
clipd [run]
```

后台服务

```bash
# 创建服务
sudo clipd install
# 启动服务
sudo clipd start
# 暂停服务
sudo clipd pause
# 恢复服务
sudo clipd resume
# 停止服务
sudo clipd stop
# 重启服务
sudo clipd restart
# 删除服务
sudo clipd uninstall
```
