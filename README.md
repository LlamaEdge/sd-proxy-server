# StableDiffusion Proxy Server

## Usage

- Start proxy server

  ```bash
  wasmedge --dir .:. ./target/wasm32-wasip1/release/sd-proxy-server.wasm
  ```

- Register sd server

  ```bash
  curl --location 'http://localhost:8080/admin/register/image' \
  --header 'Content-Type: text/plain' \
  --data 'http://localhost:7860'
  ```

  If the command runs successfully, the following message will be displayed:

  ```json
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
