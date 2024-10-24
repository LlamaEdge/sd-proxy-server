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

#### Request body

- **prompt** (string): A text description of the desired image.
- **negative_prompt** (string, optional): A text description of what the image should not contain.
- **seed** (i64, optional): Seed for the random number generator. Negative value means to use random seed. Default is -1.
- **subseed** (i64, optional): Subseed for the image generation. Defaults to -1.
- **subseed_strength** (f64, optional): Subseed strength for the image generation. Defaults to 0.0.
- **seed_resize_from_h** (i64, optional): Seed resize from H. Defaults to -1.
- **seed_resize_from_w** (i64, optional): Seed resize from W. Defaults to -1.
- **sampler_name** (string, optional): Sampler name.
- **scheduler** (string, optional): Denoiser sigma scheduler. Possible values are `discrete`, `karras`, `exponential`, `ays`, `gits`. Defaults to `discrete`.
- **batch_size** (u32, optional): Number of images to generate. Default is 1.
- **n_iter** (u32, optional): Number of iterations. Defaults to 1.
- **steps** (u32, optional): Number of sample steps to take. Default is 20.
- **cfg_scale** (f64, optional): Scale factor for the model's configuration. Default is 7.0.
- **width** (u32, optional): Width of the generated image in pixel space. Default is 512.
- **height** (u32, optional): Height of the generated image in pixel space. Default is 512.
- **restore_faces** (bool, optional): Restore faces. Defaults to false.
- **tiling** (bool, optional): Tiling. Defaults to false.
- **do_not_save_samples** (bool, optional): Do not save samples. Defaults to false.
- **do_not_save_grid** (bool, optional): Do not save grid. Defaults to false.
- **eta** (f64, optional): Eta for the image generation.
- **denoising_strength** (f64, optional): Denoising strength.
- **s_min_uncond** (f64, optional): S Min Uncond.
- **s_churn** (f64, optional): S Churn.
- **s_tmax** (f64, optional): S Tmax.
- **s_tmin** (f64, optional): S Tmin.
- **override_settings** (OverrideSettings, optional): Override settings.
- **override_settings_restore_afterwards** (bool, optional): Override settings restore afterwards. Defaults to true.
- **refiner_checkpoint** (string, optional): Refiner checkpoint.
- **refiner_switch_at** (f64, optional): Refiner switch at.
- **disable_extra_networks** (bool, optional): Disable extra networks. Defaults to false.
- **firstpass_image** (string, optional): Firstpass image.
- **enable_hr** (bool, optional): Enable Hr. Defaults to false.
- **firstphase_width** (u32, optional): Firstphase Width. Defaults to 0.
- **firstphase_height** (u32, optional): Firstphase Height. Defaults to 0.
- **hr_scale** (f64, optional): Hr scale. Defaults to 2.0.
- **hr_upscaler** (string, optional): Hr Upscaler.
- **hr_second_pass_steps** (u32, optional): Hr Second Pass Steps. Defaults to 0.
- **hr_resize_x** (u32, optional): Hr Resize X. Defaults to 0.
- **hr_resize_y** (u32, optional): Hr Resize Y. Defaults to 0.
- **hr_checkpoint_name** (string, optional): Hr Checkpoint Name.
- **hr_sampler_name** (string, optional): Hr Sampler Name.
- **hr_prompt** (string, optional): Hr Prompt. Defaults to "".
- **hr_negative_prompt** (string, optional): Hr Negative Prompt. Defaults to "".
- **force_task_id** (string, optional): Force Task Id.
- **sampler_index** (string, optional): Sampler index. Defaults to "Euler".
- **send_images** (bool, optional): Send images. Defaults to true.
- **save_images** (bool, optional): Save images. Defaults to false.
- **alwayson_scripts** (AlwaysOnScripts, optional): Alwayson scripts.

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
