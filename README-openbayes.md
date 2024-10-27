# Openbayes notes

## Local files

* pre-prepared scripts
* wasm file for the sd proxy
* the webui directory with openai and model files

```
cp /openbayes/input/input0/* .
```

## Install OS and Python deps

```
apt install -y wget git libgl1 libglib2.0-0 curl
apt install -y python3.10 python3.10-dev
pip install python-multipart
```

## Install WasmEdge

```
bash install_v2_cn.sh -v 0.14.1
```

## Install Gaia

```
bash install-gaia-cn.sh
```

## Install stable-diffusion-webui

```
chmod +x webui-cn.sh
bash webui-cn.sh -f --api --no-download-sd-model
```

## Connect with port mapping

```
ssh -L 7860:localhost:7860 root@ssh.openbayes.com -p12345
```

Open the browser

```
http://localhost:7860/
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
nohup bash webui-cn.sh -f --api --no-download-sd-model &
```

## Connect the two servers

```
curl --location 'http://localhost:8080/admin/register/image' \
--header 'Content-Type: text/plain' \
--data 'http://localhost:7860'
```

## Local test

```
cp /openbayes/input/input0/req.json .
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


