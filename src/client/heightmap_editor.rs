use imgui::*;
use na::Vector3;
use std::collections::HashSet;

use crate::gpu_obj::heightmap_gpu;
use noise::{NoiseFn, Seedable};

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Raise,
    Flatten,
    Median,
    Noise,
    Blur,
}

pub struct State {
    pub map_path: String,
    pub pen_radius: u32,
    pub pen_strength: f32,
    pub mode: Mode,
    noise: noise::Perlin,
    noise_freq: f64,
    min_z: f32,
    max_z: f32,
}

impl State {
    pub fn new() -> Self {
        State {
            map_path: "src/asset/map/map_example".to_owned(),
            pen_radius: 30,
            pen_strength: 2.0,
            mode: Mode::Raise,
            noise: noise::Perlin::new().set_seed(0),
            noise_freq: 10.0,
            min_z: 0.0,
            max_z: heightmap_gpu::MAX_Z,
        }
    }

    pub fn draw_ui(&mut self, ui: &Ui, heightmap_gpu: &mut heightmap_gpu::HeightmapGpu) {
        let pen_radius = &mut self.pen_radius;
        let pen_strength = &mut self.pen_strength;
        let mode = &mut self.mode;
        let noise_freq = &mut self.noise_freq;
        let noise_seed: &mut i32 = &mut (self.noise.seed() as i32);
        let mut update_noise = false;

        let min_z = &mut self.min_z;
        let max_z = &mut self.max_z;
        let edit_height_window = imgui::Window::new(im_str!("Heightmap editor"));
        edit_height_window
            .size([400.0, 300.0], imgui::Condition::FirstUseEver)
            .position([3.0, 415.0], imgui::Condition::FirstUseEver)
            .collapsed(false, imgui::Condition::FirstUseEver)
            .build(&ui, || {
                ui.radio_button(im_str!("Raise/Lower"), mode, Mode::Raise);
                ui.radio_button(im_str!("Flatten/Unflatten"), mode, Mode::Flatten);
                ui.radio_button(im_str!("Median"), mode, Mode::Median);
                ui.radio_button(im_str!("Blur"), mode, Mode::Blur);
                ui.radio_button(im_str!("Noise"), mode, Mode::Noise);

                if mode == &mut Mode::Noise {
                    imgui::Slider::new(im_str!("noise frequency"), 0.0_f64..=200.0)
                        .power(3.0)
                        .build(&ui, noise_freq);

                    update_noise = ui
                        .drag_int(im_str!("noise seed"), noise_seed)
                        .min(0)
                        .build();
                    ui.separator();
                } else {
                    ui.separator();
                }

                imgui::Slider::new(im_str!("pen radius"), 1..=1000).build(&ui, pen_radius);
                imgui::Slider::new(im_str!("pen strength"), 0.0..=10.0).build(&ui, pen_strength);
                ui.separator();

                imgui::Slider::new(im_str!("min height"), 0.0..=heightmap_gpu::MAX_Z)
                    .build(&ui, min_z);
                imgui::Slider::new(im_str!("max height"), 0.0..=heightmap_gpu::MAX_Z)
                    .build(&ui, max_z);

                if ui.small_button(im_str!("Save")) {
                    Self::save(heightmap_gpu, "src/asset/map/map_example");
                }

                if ui.small_button(im_str!("Clear")) {
                    for i in 0..heightmap_gpu.phy.width * heightmap_gpu.phy.height {
                        heightmap_gpu.phy.texels[i as usize] = 50.0;
                    }
                    heightmap_gpu.update_rect(
                        0 as u32,
                        0 as u32,
                        heightmap_gpu.phy.width as u32,
                        heightmap_gpu.phy.height as u32,
                    );
                }

                if ui.small_button(im_str!("Load")) {
                    Self::load(heightmap_gpu, "src/asset/map/map_example");
                }
            });

        // let window_selector = imgui::Window::new(im_str!("Map Selector"));
        // window_selector
        //     .size([400.0, 200.0], imgui::Condition::FirstUseEver)
        //     .position([400.0, 3.0], imgui::Condition::FirstUseEver)
        //     .collapsed(false, imgui::Condition::FirstUseEver)
        //     .build(&ui, || {
        //         // Self::visit_dirs_for_selection(ui);
        //     });

        self.max_z = max_z.max(*min_z);
        if update_noise {
            self.noise = self.noise.set_seed(*noise_seed as u32);
        }
    }

    pub fn handle_user_input(
        &self,
        mouse_pressed: &HashSet<winit::event::MouseButton>,
        mouse_world_pos: &Vector3<f32>,
        heightmap_gpu: &mut heightmap_gpu::HeightmapGpu,
    ) {
        log::trace!("heightmap_editor handle_user_input");
        {
            let pen_strength = self.pen_strength
                * if mouse_pressed.contains(&winit::event::MouseButton::Left) {
                    1.0
                } else if mouse_pressed.contains(&winit::event::MouseButton::Right) {
                    -1.0
                } else {
                    0.0
                };

            if pen_strength != 0.0 {
                let (x, y) = (mouse_world_pos.x, mouse_world_pos.y);

                let middle_i = x.floor() as i32;
                let middle_j = y.floor() as i32;

                let pen_size = self.pen_radius as i32;
                let pen_size2 = pen_size * pen_size;

                let min_i = (middle_i - pen_size).max(0);
                let min_j = (middle_j - pen_size).max(0);

                let max_i = (middle_i + pen_size).min(heightmap_gpu.phy.width as i32 - 1);
                let max_j = (middle_j + pen_size).min(heightmap_gpu.phy.height as i32 - 1);

                let size_i = max_i - min_i + 1;
                let size_j = max_j - min_j + 1;

                if size_i > 0 && size_j > 0 {
                    //let start = std::time::Instant::now();

                    let mut pixels = Vec::with_capacity((size_i * size_j) as usize);
                    for j in min_j..=max_j {
                        for i in min_i..=max_i {
                            let falloff = 1.0
                                - (i32::pow(i - middle_i, 2) + i32::pow(j - middle_j, 2)) as f32
                                    / pen_size2 as f32;

                            pixels.push((
                                i,
                                j,
                                (i + j * heightmap_gpu.phy.width as i32) as usize,
                                falloff.max(0.0),
                            ));
                        }
                    }

                    match self.mode {
                        Mode::Raise => {
                            for (_, _, index, falloff) in pixels {
                                let power = pen_strength * falloff;
                                heightmap_gpu.phy.texels[index] = (heightmap_gpu.phy.texels[index]
                                    + power)
                                    .min(self.max_z)
                                    .max(self.min_z);
                            }
                        }
                        Mode::Flatten => {
                            let mut average = 0.0;
                            for (_, _, index, _) in &pixels {
                                let z = heightmap_gpu.phy.texels[*index];
                                average += z;
                            }
                            average /= (size_i * size_j) as f32;
                            for (_, _, index, falloff) in pixels {
                                let power = (pen_strength * falloff) / 50.0;
                                let z = heightmap_gpu.phy.texels[index] * (1.0 - power)
                                    + average * power;
                                heightmap_gpu.phy.texels[index] = z.min(self.max_z).max(self.min_z);
                            }
                        }
                        Mode::Noise => {
                            for (i, j, index, falloff) in pixels {
                                let power = pen_strength
                                    * falloff
                                    * self.noise.get([
                                        (0.001 * self.noise_freq) * i as f64,
                                        (0.001 * self.noise_freq) * j as f64,
                                    ]) as f32;

                                heightmap_gpu.phy.texels[index] = (heightmap_gpu.phy.texels[index]
                                    + power)
                                    .min(self.max_z)
                                    .max(self.min_z);
                            }
                        }
                        Mode::Median => {
                            let mut new_pix = Vec::new();
                            for (i, j, index, _) in pixels {
                                let power = pen_strength / 10.0;

                                let kernel = 4;
                                let mut acc = Vec::new();

                                for ti in (-kernel + i).max(0)
                                    ..=(kernel + i).min(heightmap_gpu.phy.width as i32 - 1)
                                {
                                    for tj in (-kernel + j).max(0)
                                        ..=(kernel + j).min(heightmap_gpu.phy.height as i32 - 1)
                                    {
                                        let tindex =
                                            (ti + tj * heightmap_gpu.phy.width as i32) as usize;
                                        acc.push(
                                            (heightmap_gpu.phy.texels[tindex] * 1000.0 * 1000.0)
                                                .floor()
                                                as i128,
                                        );
                                    }
                                }
                                acc.sort();
                                new_pix.push((
                                    index,
                                    heightmap_gpu.phy.texels[index] * (1.0 - power)
                                        + power * (acc[acc.len() / 2] as f64 / 1000000.0) as f32,
                                ));
                            }
                            for (index, z) in new_pix {
                                heightmap_gpu.phy.texels[index] = z.min(self.max_z).max(self.min_z);
                            }
                        }
                        Mode::Blur => {
                            let mut new_pix = Vec::new();
                            for (i, j, index, falloff) in pixels {
                                let power = pen_strength * falloff / 10.0;

                                let kernel = 1;
                                let mut acc = 0.0;
                                let mut tap = 0;

                                for ti in (-kernel + i).max(0)
                                    ..=(kernel + i).min(heightmap_gpu.phy.width as i32 - 1)
                                {
                                    for tj in (-kernel + j).max(0)
                                        ..=(kernel + j).min(heightmap_gpu.phy.height as i32 - 1)
                                    {
                                        tap += 1;
                                        let tindex =
                                            (ti + tj * heightmap_gpu.phy.width as i32) as usize;
                                        acc += heightmap_gpu.phy.texels[tindex];
                                    }
                                }
                                let z = heightmap_gpu.phy.texels
                                    [(i + j * heightmap_gpu.phy.width as i32) as usize]
                                    * (1.0 - power)
                                    + power * (acc / tap as f32);
                                new_pix.push((index, z));
                            }
                            for (index, z) in new_pix {
                                heightmap_gpu.phy.texels[index] = z.min(self.max_z).max(self.min_z);
                            }
                        }
                    }

                    heightmap_gpu.update_rect(
                        min_i as u32,
                        min_j as u32,
                        size_i as u32,
                        size_j as u32,
                    );
                    //                    println!("handle hei took {}", start.elapsed().as_micros());
                }
            }
        }
    }

    pub fn save(heightmap_gpu: &heightmap_gpu::HeightmapGpu, path: &str) {
        use std::fs::File;
        use std::io::BufWriter;
        use std::path::Path;

        let height_path = format!("{}/height.png", path);
        let height_path = Path::new(&height_path);
        let file = File::create(height_path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(
            w,
            heightmap_gpu.phy.width as u32,
            heightmap_gpu.phy.height as u32,
        );
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();

        let data: Vec<u8> = heightmap_gpu
            .phy
            .texels
            .iter()
            .map(|e| ((e / 511.0).min(1.0).max(0.0) * 65535.0) as u16)
            .flat_map(|e| vec![(e >> 8) as u8, e as u8])
            .collect();
        //        let data = &data[..] ;//[255, 0, 0, 255, 0, 0, 0, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.
        writer.write_image_data(&data).unwrap(); // Save

        let json_path = &format!("{}/data.json", path);

        use std::fs::OpenOptions;
        use std::io::prelude::*;
        use std::io::BufReader;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(json_path)
            .unwrap();
        let mut buf_w = BufWriter::new(file);
        serde_json::to_writer_pretty(buf_w, &heightmap_gpu.phy.data);
    }

    pub fn load(heightmap_gpu: &mut heightmap_gpu::HeightmapGpu, path: &str) {
        use byteorder::{BigEndian, ReadBytesExt};
        use std::fs::File;
        use std::io::Cursor;
        let height_path = format!("{}/height.png", path);
        let mut decoder = png::Decoder::new(File::open(&height_path).unwrap());
        decoder.set_transformations(png::Transformations::IDENTITY);
        let (info, mut reader) = decoder.read_info().unwrap();
        log::debug!("info: {:?}", info.width);
        log::debug!("height: {:?}", info.height);
        log::debug!("bit depth: {:?}", info.bit_depth);
        log::debug!("buffer size: {:?}", info.buffer_size());
        let mut buf = vec![0; info.buffer_size()];
        reader.next_frame(&mut buf).unwrap();
        // Transform buffer into 16 bits slice.
        let mut buffer_u16 = vec![0; (info.width * info.height) as usize];
        let mut buffer_cursor = Cursor::new(buf);
        buffer_cursor
            .read_u16_into::<BigEndian>(&mut buffer_u16)
            .unwrap();

        for i in 0..heightmap_gpu.phy.width * heightmap_gpu.phy.height {
            heightmap_gpu.phy.texels[i as usize] =
                buffer_u16[i as usize] as f32 / (65535.0 / 511.0);
        }
        heightmap_gpu.update_rect(
            0 as u32,
            0 as u32,
            heightmap_gpu.phy.width as u32,
            heightmap_gpu.phy.height as u32,
        );
    }
}
