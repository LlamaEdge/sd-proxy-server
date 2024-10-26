# Openbayes notes

## Local files

* pre-prepared scripts
* wasm file for the sd proxy
* model file

```
cp /openbayes/input/input0/* .
```

## Install OS packages

```
apt-get update
apt-get upgrade
apt install -y wget git libgl1 libglib2.0-0 curl
```

## Install WasmEdge

```
bash install_v2_cn.sh -v 0.14.1
```

## Install Gaia

```
bash install-gaia-cn.sh
```

## Install Python

```
# Ubuntu 20.04
add-apt-repository ppa:deadsnakes/ppa
apt update
apt install -y python3.10 python3.10-dev
pip install python-multipart
```

## Install stable-diffusion-webui

```
chmod +x webui-cn.sh
bash webui-cn.sh -f --api --no-download-sd-model
```

## Use the downloaded models

```
cp *safetensors  stable-diffusion-webui/models/Stable-diffusion/
```

## Add extension

Follow the steps in [this guide](https://github.com/Mikubill/sd-webui-controlnet?tab=readme-ov-file#installation). Use China proxy for GitHub.

```
https://mirror.ghproxy.com/https://github.com/Mikubill/sd-webui-controlnet.git
```

## Start the proxy

```
nohup wasmedge --dir .:. sd-proxy-server.wasm &
```

## Start webui server

```
nohup bash webui.sh -f --api &
```

## Connect the two servers

```
curl --location 'http://localhost:8080/admin/register/image' \
--header 'Content-Type: text/plain' \
--data 'http://localhost:7860'
```

## Local test

```
cp /openbayes/input0/req.json .
curl -o output.json -X POST -H "Content-Type: application/json" -d @req.json http://localhost:8080/v1/images/generations
```

## Start frp

Add to `~/gaianet/gaia-frp/frpc.toml`

```
[[proxies]]
name = "controlnet-image.us.gaianet.network"
type = "http"
localPort = 8080
subdomain = "controlnet-image"
```

Start the frp service.

```
nohup /root/gaianet/bin/frpc -c /root/gaianet/gaianet-frp/frpc.toml &
```

## Public test

```
curl -o output.json -X POST -H "Content-Type: application/json" -d @req.json https://controlnet-image.us.gaianet.network/v1/images/generations
```


