# StableDiffusion Proxy Server

> [!NOTE]
> The project is still under active development. The existing features still need to be improved and more features will be added in the future.

## Setup

- Install dependencies

  ```bash
  # Ubuntu 20.04
  sudo add-apt-repository ppa:deadsnakes/ppa
  sudo apt update
  sudo apt install -y python3.10 python3.10-dev
  sudo apt install -y python3.10-venv
  sudo apt install -y wget git libgl1 libglib2.0-0 curl
  ```

- Install sd-webui

  ```bash
  # Download script
  wget -q https://raw.githubusercontent.com/AUTOMATIC1111/stable-diffusion-webui/master/webui.sh

  # Make script executable
  chmod +x webui.sh

  # Run script
  bash webui.sh -f
  ```

- Download models (Optional)

  ```bash
  curl -L https://huggingface.co/second-state/waiANINSFWPONYXL_v90-GGUF/resolve/main/waiANINSFWPONYXL_v90-f16.safetensors -o ./stable-diffusion-webui/models/Stable-diffusion/waiANINSFWPONYXL_v90-f16.safetensors
  ```

- Install wasmedge

  ```bash
  curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install_v2.sh | bash -s -- -v 0.14.1
  ```

- Download proxy server wasm app

  ```bash
  curl -LO https://github.com/LlamaEdge/sd-proxy-server/releases/latest/download/sd-proxy-server.wasm
  ```

## Usage

- Start proxy server

  ```bash
  wasmedge --dir .:. sd-proxy-server.wasm
  ```

  > `sd-proxy-server` will use `8080` port by default. You can change the port by adding `--port <port>`.

- Start downstream sd server

  ```bash
  ./webui.sh --api

  # or

  bash webui.sh -f --api
  ```

- Register downstream sd server

  ```bash
  curl --location 'http://localhost:8080/admin/register/image' \
  --header 'Content-Type: text/plain' \
  --data 'http://localhost:7860'
  ```

  If the command runs successfully, the following message will be displayed:

  ```text
  Registered server url: http://localhost:7860/
  ```

- Send a text-to-image request to the proxy server

  ```bash
  curl -X POST 'http://localhost:8080/v1/images/generations' \
    --header 'Content-Type: application/json' \
    --data '{
        "model": "sd-v1.4",
        "prompt": "A cute baby sea otter"
    }'
  ```

  If the command runs successfully, the following message will be displayed:

  ```json
  [
    {
        "b64_json": "\"iVBORw0KGgoAAAANSUhEUgAAAgAAAAIACAIAAAB7GkOtAAAAxHRFW...\"",
        "prompt": "\"A cute baby sea otter\""
     }
  ]
  ```

- Send an image-to-image request to the proxy server

  ```bash
  curl --location 'http://localhost:10086/v1/images/edits' \
    --form 'image=@"/path/to/your_image.png"' \
    --form 'prompt="your prompt"'
  ```

  If the command runs successfully, the following message will be displayed:

  ```json
  [
    {
        "b64_json": "\"iVBORw0KGgoAAAANSUhEUgAAAgAAAAIACAIAAAB7GkOtAAAAxHRFW...\"",
        "prompt": "\"your prompt\""
     }
  ]
  ```
