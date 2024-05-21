use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use three_d::*;

#[derive(ValueEnum, Clone, Debug)]
enum ArgCamera {
	#[value()]
	Orthographic,
	#[value()]
	Perspective,
	#[value()]
	Wiki,
}

#[derive(Parser, Clone, Debug)]
struct Args {
	/// Minecraft json model file to display
	#[clap()]
	pub json_file: PathBuf,
	/// Camera field type to use
	#[clap(value_enum, short, long, default_value = "perspective")]
	pub camera: ArgCamera,
	/// Path to immediately save a screenshot to upon open
	#[clap(short, long)]
	pub screenshot: Option<PathBuf>,
	/// Window width
	#[clap(long, default_value = "640")]
	pub width: u32,
	/// Window height
	#[clap(long, default_value = "640")]
	pub height: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
	run().await?;
	Ok(())
}

async fn run() -> anyhow::Result<()> {
	let args = Args::parse();

	let window = Window::new(WindowSettings {
		title: "MC JSON Renderer".to_string(),
		max_size: Some((args.width, args.height)),
		..Default::default()
	})?;

	let context = window.gl();

	let mut camera = match args.camera {
		ArgCamera::Perspective => Camera::new_perspective(
			window.viewport(),
			//vec3(0.0, 0.0, 16.0 * 2.0),
			vec3(48.0, 40.0, -32.0),
			vec3(8.0, 8.0, 8.0),
			vec3(0.0, 1.0, 0.0),
			degrees(60.0),
			0.1,
			256.0,
		),
		ArgCamera::Orthographic => Camera::new_orthographic(
			window.viewport(),
			vec3(48.0, 40.0, -32.0),
			vec3(8.0, 8.0, 8.0),
			vec3(0.0, 1.0, 0.0),
			32.0,
			0.1,
			256.0,
		),
		ArgCamera::Wiki => Camera::new_orthographic(
			window.viewport(),
			vec3(-32.0, 40.0, -32.0),
			vec3(8.0, 8.0, 8.0),
			vec3(0.0, 1.0, 0.0),
			32.0,
			0.1,
			256.0,
		),
	};

	let mc_json_model = mc_json_stuff::McModelJson::parse_json_model_from_reader(std::fs::File::open(&args.json_file)?)?;
	let (cpu_mesh, tex_albedo) = mc_json_model.to_cpu_mesh(args.json_file.parent().expect("JSON base path must exist"))?;
	eprintln!("The MCJson model `{:?}` has {} vertices and {} indices", args.json_file, cpu_mesh.positions.len(), cpu_mesh.indices.len().unwrap_or(0));
	let gpu_mesh = Mesh::new(&context, &cpu_mesh);
	// let white_cpu_texture = CpuTexture {
	// 	name: "white".to_string(),
	// 	data: TextureData::RU8(vec![255]),
	// 	width: 1,
	// 	height: 1,
	// 	min_filter: Default::default(),
	// 	mag_filter: Default::default(),
	// 	mip_map_filter: None,
	// 	wrap_s: Wrapping::Repeat,
	// 	wrap_t: Wrapping::Repeat,
	// };
	let cpu_mat = CpuMaterial {
		name: "atlas".to_string(),
		albedo: Srgba::WHITE,
		albedo_texture: Some(tex_albedo),
		metallic: 0.0,
		roughness: 0.0,
		occlusion_metallic_roughness_texture: None,
		metallic_roughness_texture: None,
		occlusion_strength: 0.0,
		occlusion_texture: None,
		normal_scale: 0.0,
		normal_texture: None,
		emissive: Default::default(),
		emissive_texture: None,
		alpha_cutout: None,
		lighting_model: LightingModel::Phong,
		index_of_refraction: 0.0,
		transmission: 0.0,
		transmission_texture: None,
	};
	// let mut mat = PhysicalMaterial::new(&context, &cpu_mat);
	let mat = ColorMaterial {
		render_states: RenderStates {
			write_mask: WriteMask::COLOR_AND_DEPTH,
			depth_test: DepthTest::Less,
			blend: Blend::STANDARD_TRANSPARENCY, // Careful, STANDARD_TRANSPARENCY doesn't work right on WebGL if compiling for the web
			cull: Cull::Back,
		},
		..ColorMaterial::new_transparent(&context, &cpu_mat)
	};
	let model = Gm::new(gpu_mesh, mat);

	if let Some(screenshot_path) = &args.screenshot {
		let mut texture = Texture2D::new_empty::<[u8; 4]>(
			&context,
			args.width,
			args.height,
			Interpolation::Nearest,
			Interpolation::Nearest,
			None,
			Wrapping::ClampToEdge,
			Wrapping::ClampToEdge,
		);
		let mut depth_texture = DepthTexture2D::new::<f32>(
			&context,
			args.width,
			args.height,
			Wrapping::ClampToEdge,
			Wrapping::ClampToEdge,
		);
		camera.set_viewport(Viewport::new_at_origo(args.width, args.height));
		let colors = RenderTarget::new(
			texture.as_color_target(None),
			depth_texture.as_depth_target(),
		)
			// Clear color and depth of the render target
			.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 0.0, 1.0))
			// Render the triangle with the per vertex colors defined at construction
			.render(&camera, &model, &[])
			// Read out the colors from the render target
			.read_color::<[u8; 4]>();
		let colors = colors.into_iter().flatten().collect::<Vec<u8>>();
		if let Err(error) = image::save_buffer(screenshot_path, &colors, args.width, args.height, image::ColorType::Rgba8) {
			eprintln!("Failed to save screenshot to {screenshot_path:?}: {error}");
		} else {
			eprintln!("Saved screenshot to {screenshot_path:?}");
		}
		return Ok(());
	}

	// let light = SpotLight::new(&context, 0.8, Srgba::WHITE, &Vec3::new(16.0, 32.0, 16.0), &Vec3::new(-16.0, -32.0, -16.0), Deg(90.0), Attenuation::default());
	// let light = AmbientLight::new(&context, 0.7, Srgba::WHITE);
	// let ambient = AmbientLight::new(&context, 0.4, Srgba::GREEN);
	// let light = DirectionalLight::new(&context, 2.0, Srgba::BLUE, &vec3(0.0, -1.0, -10.0));

	let mut orbit_control = OrbitControl::new(*camera.target(), 17.0, 128.0);

	window.render_loop(move |mut frame_input| {
		let mut redraw = frame_input.first_frame;
		redraw |= camera.set_viewport(frame_input.viewport);
		redraw |= orbit_control.handle_events(&mut camera, &mut frame_input.events);
		redraw |= true; // Always redraw for now

		if redraw {
			let target = frame_input.screen();

			target
				.clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
				.render(
					&camera, [&model], &[],
				);
		}

		FrameOutput {
			swap_buffers: redraw,
			..Default::default()
		}
	});

	Ok(())
}
