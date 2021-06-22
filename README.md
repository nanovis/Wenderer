# Wenderer
WebGPU-based SciVis Renderer

## Compilation Guide
For installing Rust, please see [official guide](https://www.rust-lang.org/learn/get-started), which is an oneliner that is different according to OSs.

For running the code, run one of the two lines in the source code directory.
```shell
# debug profile
cargo run
# release profile
cargo run --release
```
The dependencies are managed automatically by `cargo` according to `Cargo.toml`.

## Interactions
For now, we have simple interactions:
* Press `A`, `D` to rotate camera.
* Press `W`, `S` to zoom in and out.

## Used WebGPU Features
* Textures (1D, 2D, 3D)
* Multi-pass Rendering
* Front-face and back-face Rendering/ Depth Testing
* Render Buffers
* Multisampling
* `wgsl` shaders

## TODOs
* Ray jittering
* Better camera
* Configurable transfer functions
* Configurable volumes

## Reference and Acknowledgements
* We thank sotrh@Github for his detailed and nicely-written [tutorial](https://sotrh.github.io/learn-wgpu/) about WebGPU and his patience on answering WebGPU questions.
* We thank kvark@Github for helping resolve a key issue in our implementation in the [Github Discussion in wgpu](https://github.com/gfx-rs/wgpu/discussions/1491).
* For the `skewed_head.dat`, we are finding the reference to it, so please do **not** distribute it for now.