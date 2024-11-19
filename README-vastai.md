# Setup guide for vast.ai

## Starting a server

Select IMAGE template `stable-diffusion-webui-cuda:latest`

It comes default: Python 3.10.12

## Set up software

Install dependencies

```
apt install -y wget git libgl1 libglib2.0-0 curl
pip install insightface
```

Download model in the `/workspace` folder.

```
curl -L https://huggingface.co/second-state/waiANINSFWPONYXL_v90-GGUF/resolve/main/waiANINSFWPONYXL_v90-f16.safetensors -o ./stable-diffusion-webui/models/Stable-diffusion/waiANINSFWPONYXL_v90-f16.safetensors
```

Install GaiaNet. You should expect to see errors of missing CUDA BLAS libs. Ignore them.

```
curl -sSfL 'https://github.com/GaiaNet-AI/gaianet-node/releases/latest/download/install.sh' | bash
```

Overwrite WasmEdge with a regular version.

```
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash
source /root/.bashrc
```

## Set up WebUI

In the `/workspace` folder. Start WebUI on the CLI. Make sure that all dependencies are installed and it starts properly.

```
cd stable-diffusion-webui
bash webui.sh -f --api
... ...
Running on local URL:  http://127.0.0.1:7861
```

On your local computer, check `http://localhost:7861` and go to `extensions` to see that the `control-net` is active. Generate an image using the UI.

## Start up production

Go to the server and use `CRTL-C` to quit the WebUI app. Then start it in the background.

```
nohup bash webui.sh -f --api &
```

Download and run the proxy server.

```
cd ..
curl -LO https://github.com/LlamaEdge/sd-proxy-server/releases/latest/download/sd-proxy-server.wasm
nohup wasmedge --dir .:. sd-proxy-server.wasm --port 8081 &
```

Connect WebUI to proxy server.

```
curl --location 'http://localhost:8081/admin/register/image' \
--header 'Content-Type: text/plain' \
--data 'http://127.0.0.1:7861'
```

Edit the `/root/gaianet/gaia-frp/frpc.toml` file and change the domain to `gaia.domains` and port to `8081`. Start the frpc client.

```
nohup /root/gaianet/bin/frpc -c /root/gaianet/gaia-frp/frpc.toml &
```

## Test it

```
curl -o output.json -X POST -H "Content-Type: application/json" -d @req.json https://0xda0ccfa1bfe1be37066e3a74a25cea650c733da2.gaia.domains/v1/images/generations

  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
100 6507k  100 3141k  100 3365k   313k   335k  0:00:10  0:00:10 --:--:--  813k
```
