# StableDiffusion Proxy Server

> [!NOTE]
> The project is still under active development. The existing features still need to be improved and more features will be added in the future.

## Setup

- Install dependencies

  - Add Python PPA

    ```bash
    # Ubuntu 20.04
    sudo add-apt-repository ppa:deadsnakes/ppa
    sudo apt update
    ```

  - Install Python 3.10 (Optional)

    If Python 3.10 is already installed, you can skip this step. Use `python3 --version` or `python --version` to check if Python 3.10 is installed.

    ```bash
    sudo apt install -y python3.10 python3.10-dev
    ```

  - Install Python 3.10 venv

    ```bash
    sudo apt install -y python3.10-venv
    ```

  - Install other dependencies

    ```bash
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

  Assume that the downstream sd server is running on `http://localhost:7860`. Then you can register it by the following command:

  ```bash
  curl --location 'http://localhost:8080/admin/register/image' \
  --header 'Content-Type: text/plain' \
  --data 'http://localhost:7860'
  ```

  If the command runs successfully, the following message will be displayed:

  ```json
  {
    "message": "URL registered successfully",
    "url": "http://localhost:7860/"
  }
  ```

- Send a text-to-image request to the proxy server

  ```bash
  curl -X POST http://localhost:8080/v1/images/generations \
  --header 'Content-Type: application/json' \
  --data '{
    "prompt": "1girl,intricate,highly detailed,Mature,seductive gaze,teasing expression,sexy posture,solo,Moderate breasts,Charm,alluring,Hot,tsurime,lipstick,stylish_pose,long hair,long_eyelashes,black hair,bar,dress,",
    "negative_prompt": "",
    "seed": -1,
    "batch_size": 1,
    "steps": 20,
    "scheduler": "Karras",
    "cfg_scale": 7,
    "width": 540,
    "height": 960,
    "restore_faces": false,
    "tiling": false,
    "override_settings": {
        "sd_model_checkpoint": "waiANINSFWPONYXL_v90.safetensors"
    },
    "sampler_index": "DPM++ 2M",
    "alwayson_scripts": {
        "controlnet": {
            "args": [
                {
                    "enabled": true,
                    "pixel_perfect": true,
                    "image": "......"
                    "module": "reference_only",
                    "guidance_start": 0,
                    "guidance_end": 0.2
                }
            ]
        }
    }
  }'
  ```

  If the command runs successfully, the following message will be displayed:

  ```json
  [
    {
        "b64_json": "iVBORw0KGgoAAAANSUhEUgAAAhgAAAP......",
        "prompt": "1girl,intricate,highly detailed,Mature,seductive gaze,teasing expression,sexy posture,solo,Moderate breasts,Charm,alluring,Hot,tsurime,lipstick,stylish_pose,long hair,long_eyelashes,black hair,bar,dress,"
     }
  ]
  ```
