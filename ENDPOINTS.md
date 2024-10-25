# Endpoints

sd-proxy-server provides two endpoints for image generation and editing. The following sections describe how to use these endpoints.

> [!NOTE]
> The project is still under active development. The existing features still need to be improved and more features will be added in the future.

## Business Endpoint

### Create Image

```bash
POST http://localhost:{port}/v1/images/generations
```

Creates an image given a prompt.

#### Request Parameters

```json
{
  # (string) A text description of the desired image.
  "prompt": "",
  # (string, optional) A text description of what the image should not contain. Defaults to "".
  "negative_prompt": "",
  # (i64, optional) Seed for the random number generator. Negative value means to use random seed. Default is -1.
  "seed": -1,
  # (i64, optional) Subseed for the image generation. Defaults to -1.
  "subseed": -1,
  # (f64, optional) Subseed strength for the image generation. Defaults to 0.0.
  "subseed_strength": 0.0,
  # (i64, optional) Seed resize from H. Defaults to -1.
  "seed_resize_from_h": -1,
  # (i64, optional) Seed resize from W. Defaults to -1.
  "seed_resize_from_w": -1,
  # (string, optional) Sampler name. Possible values are `Euler`, `Euler a`, `LMS`, `Heun`, `DPM++ 2M`, `DPM++ 2M Karras`, `DPM2`, `DPM2 a`, `DPM++ SDE`, `DPM++ SDE Karras`, `LMS Karras`, `DPM2 Karras`, `DPM++ SDE Karras`. Defaults to `Euler`.
  "sampler_name": "Euler",
  # (string, optional) Denoiser sigma scheduler. Possible values are `discrete`, `karras`, `exponential`, `ays`, `gits`. Defaults to `discrete`.
  "scheduler": "discrete",
  # (u32, optional) Number of images to generate. Default is 1.
  "batch_size": 1,
  # (u32, optional) Number of iterations. Defaults to 1.
  "n_iter": 1,
  # (u32, optional) Number of sample steps to take. Default is 20.
  "steps": 20,
  # (f64, optional) Scale factor for the model's configuration. Default is 7.0.
  "cfg_scale": 7.0,
  # (u32, optional) Width of the generated image in pixel space. Default is 512.
  "width": 512,
  # (u32, optional) Height of the generated image in pixel space. Default is 512.
  "height": 512,
  # (bool, optional) Restore faces. Defaults to false.
  "restore_faces": false,
  # (bool, optional) Tiling. Defaults to false.
  "tiling": false,
  # (bool, optional) Do not save samples. Defaults to false.
  "do_not_save_samples": false,
  # (bool, optional) Do not save grid. Defaults to false.
  "do_not_save_grid": false,
  # (f64, optional) Eta for the image generation.
  "eta": null,
  # (f64, optional) Denoising strength.
  "denoising_strength": null,
  # (f64, optional) S Min Uncond.
  "s_min_uncond": null,
  # (f64, optional) S Churn.
  "s_churn": null,
  # (f64, optional) S Tmax.
  "s_tmax": null,
  # (f64, optional) S Tmin.
  "s_tmin": null,
  # (OverrideSettings, optional) Override settings.
  "override_settings": {
    # (string, optional) SD Model Checkpoint.
    "sd_model_checkpoint": ""
  },
  # (bool, optional) Override settings restore afterwards. Defaults to true.
  "override_settings_restore_afterwards": true,
  # (string, optional) Refiner checkpoint.
  "refiner_checkpoint": null,
  # (f64, optional) Refiner switch at.
  "refiner_switch_at": null,
  # (bool, optional) Disable extra networks. Defaults to false.
  "disable_extra_networks": false,
  # (bool, optional) Enable Hr. Defaults to false.
  "enable_hr": false,
  # (u32, optional) Firstphase Width. Defaults to 0.
  "firstphase_width": 0,
  # (u32, optional) Firstphase Height. Defaults to 0.
  "firstphase_height": 0,
  # (f64, optional) Hr scale. Defaults to 2.0.
  "hr_scale": 2.0,
  # (string, optional) Hr Upscaler.
  "hr_upscaler": null,
  # (u32, optional) Hr Second Pass Steps. Defaults to 0.
  "hr_second_pass_steps": 0,
  # (u32, optional) Hr Resize X. Defaults to 0.
  "hr_resize_x": 0,
  # (u32, optional) Hr Resize Y. Defaults to 0.
  "hr_resize_y": 0,
  # (string, optional) Hr Checkpoint Name.
  "hr_checkpoint_name": null,
  # (string, optional) Hr Sampler Name.
  "hr_sampler_name": null,
  # (string, optional) Hr Prompt. Defaults to "".
  "hr_prompt": "",
  # (string, optional) Hr Negative Prompt. Defaults to "".
  "hr_negative_prompt": "",
  # (string, optional) Sampler index. Possible values are `Euler`, `Euler a`, `LMS`, `Heun`, `DPM++ 2M`, `DPM++ 2M Karras`, `DPM2`, `DPM2 a`, `DPM++ SDE`, `DPM++ SDE Karras`, `LMS Karras`, `DPM2 Karras`, `DPM++ SDE Karras`. Defaults to `Euler`.
  "sampler_index": "Euler",
  # (bool, optional) Send images. Defaults to true.
  "send_images": true,
  # (bool, optional) Save images. Defaults to false.
  "save_images": false,
  # (AlwaysOnScripts, optional) Alwayson scripts.
  "alwayson_scripts": {
    "controlnet": {
      # (array, optional) ControlNet arguments. Defaults to empty array.
      "args": [
        {
          # (bool, optional) Enabled. Defaults to false.
          "enabled": true,
          # (bool, optional) Pixel perfect. Defaults to false.
          "pixel_perfect": true,
          # (string) Image base64 string.
          "image": ".......",
          # (string) ControlNet module.
          "module": "reference_only",
          # (f64, optional) Guidance start. Defaults to 0.0.
          "guidance_start": 0.0,
          # (f64, optional) Guidance end. Defaults to 0.2.
          "guidance_end": 0.2
        }
      ]
    }
  }
}
```

#### Example

- Text-to-image generation with reference-only control:

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

## Admin Endpoints

### List Downstream Servers

```bash
curl -X POST http://localhost:{port}/admin/servers
```

If the command runs successfully and there are registered downstream servers, the following message will be displayed:

```json
{
    "image": [
        "http://localhost:7860/"
    ]
}
```

### Register Downstream Server

```bash
curl -X POST http://localhost:{port}/admin/register/image -d "http://localhost:7860"
```

If the command runs successfully, the following message will be displayed:

```json
{
    "message": "URL registered successfully",
    "url": "http://localhost:7860/"
}
```

### Unregister Downstream Server

```bash
curl -X POST http://localhost:{port}/admin/unregister/image -d "http://localhost:7860"
```

If the command runs successfully, the following message will be displayed:

```json
{
    "message": "URL unregistered successfully",
    "url": "http://localhost:7860/"
}
```
