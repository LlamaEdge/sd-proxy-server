# Openbayes notes

## Install OS packages

```
sudo apt-get update
sudo apt-get upgrade
sudo apt install -y wget git libgl1 libglib2.0-0 curl
```

## Install Gaia

```
cp /openbayes/input0/install-gaia-cn.sh .
bash install-gaia-cn.sh
```

## Install Python

```
# Ubuntu 20.04
sudo add-apt-repository ppa:deadsnakes/ppa
sudo apt update
sudo apt install -y python3.10 python3.10-dev
```

## Get wasm

```
cp /openbayes/input0/sd-proxy-server.wasm .
```

## Install stable-diffusion-webui

```
cp /openbayes/input0/webui-cn.sh .
chmod +x webui-cn.sh
bash webui-cn.sh -f
```

## Get model

```
cp /openbayes/input0/waiANINSFWPONYXL_v90-f16.safetensors ./stable-diffusion-webui/models/Stable-diffusion/waiANINSFWPONYXL_v90-f16.safetensors
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


