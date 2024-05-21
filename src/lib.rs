use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;

use anyhow::Context as AnyContext;
use image::{GenericImage, RgbaImage};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use three_d::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McModelRotationAxis {
	X,
	Y,
	Z,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McModelDirection {
	North,
	East,
	South,
	West,
	Up,
	Down,
}

impl McModelDirection {
	pub fn get_normal(&self) -> Vec3 {
		match self {
			McModelDirection::North => vec3(0.0, 0.0, -1.0),
			McModelDirection::East => vec3(1.0, 0.0, 0.0),
			McModelDirection::South => vec3(0.0, 0.0, 1.0),
			McModelDirection::West => vec3(-1.0, 0.0, 0.0),
			McModelDirection::Up => vec3(0.0, 1.0, 0.0),
			McModelDirection::Down => vec3(0.0, -1.0, 0.0),
		}
	}

	pub fn get_shading_mult(&self) -> f32 {
		// Wtf MC really?  Can't just do something reasonable like a basic phong or something???
		// Instead of hardcoded side hell??
		match self {
			McModelDirection::North => 0.8,
			McModelDirection::East => 0.6,
			McModelDirection::South => 0.8,
			McModelDirection::West => 0.6,
			McModelDirection::Up => 1.0,
			McModelDirection::Down => 0.5,
		}
	}

	pub fn get_shading_srgba(&self) -> Srgba {
		let shading = self.get_shading_mult();
		Srgba::from([shading, shading, shading, 1.0])
	}

	// pub fn get_next_dir_around(self, axis: McModelRotationAxis) -> McModelDirection {
	// 	use McModelRotationAxis::*;
	// 	use McModelDirection::*;
	// 	match (axis, self) {
	// 		(X, North) => Down,
	// 		(X, South) => Up,
	// 		(X, East) => East,
	// 		(X, West) => West,
	// 		(X, Up) => North,
	// 		(X, Down) => South,
	//       ...
	// 	}
	// }

	// pub fn get_shading_from_face(&self, _rot: &McModelRotation) -> f32 {
	// 	// use McModelRotationAxis::*;
	// 	// fn interp(a: f32, b: f32, t: f32) -> f32 {
	// 	// 	// Just lerping for now unless someone tells me it does something otherwise like slerp or
	// 	// 	// so, not that I trust MC to do anything remotely competent like that anyway.
	// 	// 	a * (1.0 - t) + b * t
	// 	// }
	// 	// // Yes, MC only allows 22.5 degree angles.... wtf??
	// 	// let angle = ((rot.angle as f32 % 360.0) * 3600.0) as i32 / 225;
	// 	// match (angle, rot.axis) {
	// 	// 	(0, _) => self.get_shading_mult(),
	// 	// 	(1, X) => self.get_shading_normal()
	// 	// 	_ => unreachable!("unsupported angle: {} axis: {:?}", rot.angle, rot.axis),
	// 	// }
	// 	// todo!()
	// 	// As per `Omni` it just clamps to nearest face, which I'm guessing means it just keeps what
	// 	// it is by default since they always seem to be near their "natural" face anyway, plus I
	// 	// really don't see MC being well programmed enough to do anything else unless I'm shown
	// 	// otherwise...
	// 	unimplemented!("MC doesn't interpolate shading...")
	// }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McModelRotation {
	pub angle: f64,
	pub axis: McModelRotationAxis,
	pub origin: [f64; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McModelFace {
	pub uv: [f64; 4],
	pub texture: String,
	#[serde(default, skip_serializing_if = "num_traits::identities::Zero::is_zero")]
	pub rotation: i16,
	pub cullface: Option<McModelDirection>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McModelFaces {
	pub north: Option<McModelFace>,
	pub east: Option<McModelFace>,
	pub south: Option<McModelFace>,
	pub west: Option<McModelFace>,
	pub up: Option<McModelFace>,
	pub down: Option<McModelFace>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McModelElement {
	pub from: [f64; 3],
	pub to: [f64; 3],
	pub faces: McModelFaces,
	pub rotation: Option<McModelRotation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McModelDisplay {
	FirstpersonRighthand {
		rotation: [f64; 3],
		translation: [f64; 3],
		scale: [f64; 3],
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McModelJson {
	pub parent: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub display: Option<McModelDisplay>,
	pub textures: IndexMap<String, String>,
	pub elements: Vec<McModelElement>,
}


impl McModelElement {
	pub fn faces_enabled(&self) -> usize {
		let mut count = 0;
		if self.faces.north.is_some() {
			count += 1;
		}
		if self.faces.east.is_some() {
			count += 1;
		}
		if self.faces.south.is_some() {
			count += 1;
		}
		if self.faces.west.is_some() {
			count += 1;
		}
		if self.faces.up.is_some() {
			count += 1;
		}
		if self.faces.down.is_some() {
			count += 1;
		}
		count
	}

	pub fn transformation(&self) -> Mat4 {
		if let Some(rot) = &self.rotation {
			let mat = match rot.axis {
				McModelRotationAxis::X => Mat4::from_angle_x(Deg(rot.angle as f32)),
				McModelRotationAxis::Y => Mat4::from_angle_y(Deg(rot.angle as f32)),
				McModelRotationAxis::Z => Mat4::from_angle_z(Deg(rot.angle as f32)),
			};
			mat + Mat4::from_translation(Vec3::new(rot.origin[0] as f32, rot.origin[1] as f32, rot.origin[2] as f32))
		} else {
			Mat4::identity()
		}
	}
}

impl McModelJson {
	pub fn parse_json_model_slice(json_data: &[u8]) -> anyhow::Result<McModelJson> {
		Ok(serde_json::from_slice(json_data)?)
	}

	pub fn parse_json_model_from_reader(json_data: impl std::io::Read) -> anyhow::Result<McModelJson> {
		Ok(serde_json::from_reader(json_data)?)
	}

	pub fn face_count(&self) -> usize {
		let mut count = 0;
		for element in &self.elements {
			if element.faces.north.is_some() {
				count += 1;
			}
			if element.faces.east.is_some() {
				count += 1;
			}
			if element.faces.south.is_some() {
				count += 1;
			}
			if element.faces.west.is_some() {
				count += 1;
			}
			if element.faces.up.is_some() {
				count += 1;
			}
			if element.faces.down.is_some() {
				count += 1;
			}
		}
		count
	}

	pub fn to_cpu_mesh(&self, texture_base_path: &Path) -> anyhow::Result<(CpuMesh, CpuTexture)> {
		// Texture building
		let mut atlas_mappings = HashMap::with_capacity(self.textures.len());
		let err_tex;
		let texture = {
			use etagere::*;
			let mut tex = RgbaImage::new(2048, 2048);
			let mut atlas = AtlasAllocator::new(size2(2048, 2048));
			err_tex = atlas.allocate(size2(16, 16)).with_context(|| "unable to allocate not-found 16x16 space on atlas")?;
			for x in err_tex.rectangle.min.x..err_tex.rectangle.max.x {
				for y in err_tex.rectangle.min.y..err_tex.rectangle.max.y {
					tex.put_pixel(x as u32, y as u32, if (x + y) % 2 == 0 { image::Rgba([255, 0, 255, 255]) } else { image::Rgba([0, 0, 0, 255]) });
				}
			}
			for (tex_id, tex_path) in &self.textures {
				let texture_path = {
					let mut texture_path = tex_path.clone();
					texture_path.push_str(".png");
					texture_base_path.join(&texture_path)
				};
				if let Ok(tile) = image::open(&texture_path) {
					let tile = tile.to_rgba8();
					let mapping = atlas.allocate(size2(tile.width() as i32, tile.height() as i32)).with_context(|| format!("unable to store {tex_id} image on atlas from: {tex_path}"))?;
					tex.copy_from(&tile, mapping.rectangle.min.x as u32, mapping.rectangle.min.y as u32)?;
					atlas_mappings.insert(tex_id.clone(), mapping);
				} else {
					eprintln!("unable to open texture: {texture_path:?}");
				}
			}
			// image::save_buffer("atlas.png", tex.as_raw(), tex.width(), tex.height(), image::ColorType::Rgba8)?;
			CpuTexture {
				name: "atlas".to_string(),
				data: TextureData::RgbaU8(tex.pixels().map(|p| p.0).collect()),
				width: tex.width(),
				height: tex.height(),
				min_filter: Interpolation::Nearest,
				mag_filter: Interpolation::Nearest,
				mip_map_filter: None,
				wrap_s: Wrapping::ClampToEdge,
				wrap_t: Wrapping::ClampToEdge,
			}
		};

		// Mesh building
		// Don't normally do this with floats unless you understand the dangers involved
		#[derive(PartialEq)]
		struct Vec3S {
			x: f64,
			y: f64,
			z: f64,
			u: f64,
			v: f64,
			color: Srgba,
		}
		impl Eq for Vec3S {}
		impl Hash for Vec3S {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				self.x.to_bits().hash(state);
				self.y.to_bits().hash(state);
				self.z.to_bits().hash(state);
				self.u.to_bits().hash(state);
				self.v.to_bits().hash(state);
				self.color.hash(state);
			}
		}
		let mut datas = IndexMap::with_capacity(self.elements.len() * 36);
		let mut indices = Vec::with_capacity(self.elements.len() * 36);

		let mut push_pos = |x: f64, y: f64, z: f64, u: f64, v: f64, color: Srgba| {
			let pos = Vec3S { x, y, z, u, v, color };
			if let Some((idx, _, ())) = datas.get_full(&pos) {
				indices.push(idx as u32);
			} else {
				indices.push(datas.len() as u32);
				datas.insert(pos, ());
			};
		};

		let bleed = (2048.0f64 * 16.0).recip();
		let get_uv = |face: &McModelFace| -> anyhow::Result<(bool, [f64; 4])> {
			let offset = if let Some(atlas_mapping) = atlas_mappings.get(face.texture.strip_prefix('#').context("texture ID should start with '#'")?) {
				atlas_mapping.rectangle
			} else {
				err_tex.rectangle
			};
			let [u0, v0, u1, v1] = face.uv;
			let [v0, v1] = [v1, v0];
			let (flip, [u0, v0, u1, v1]) =
				match face.rotation {
					0 => (false, [u0, v0, u1, v1]),
					90 => (true, [u0, v1, u1, v0]),
					180 => (false, [u1, v1, u0, v0]),
					270 => (true, [u1, v0, u0, v1]),
					_ => anyhow::bail!("unsupported rotation: {}", face.rotation),
				};
			// TODO: Hard coded to size 16 for now, that might be by MC design, or maybe it's calculated from somewhere?
			let u0 = (offset.min.x as f64 + u0) / 2048.0;
			let v0 = (offset.min.y as f64 + v0) / 2048.0;
			let u1 = (offset.min.x as f64 + u1) / 2048.0;
			let v1 = (offset.min.y as f64 + v1) / 2048.0;
			let [u0, u1] = if u0 < u1 {
				[u0 + bleed, u1 - bleed]
			} else {
				[u0 - bleed, u1 + bleed]
			};
			let [v0, v1] = if v0 < v1 {
				[v0 + bleed, v1 - bleed]
			} else {
				[v0 - bleed, v1 + bleed]
			};
			Ok((flip, [u0, v0, u1, v1]))
		};

		for element in &self.elements {
			let (rot, origin) = if let Some(rot) = &element.rotation {
				let mat = match rot.axis {
					McModelRotationAxis::X => Matrix4::<f64>::from_angle_x(Deg(rot.angle)),
					McModelRotationAxis::Y => Matrix4::<f64>::from_angle_y(Deg(rot.angle)),
					McModelRotationAxis::Z => Matrix4::<f64>::from_angle_z(Deg(rot.angle)),
				};
				let origin = vec3(rot.origin[0], rot.origin[1], rot.origin[2]);
				(mat, origin)
			} else {
				(Matrix4::<f64>::identity(), vec3(0.0, 0.0, 0.0))
			};
			let mut push_pos = |x: f64, y: f64, z: f64, u: f64, v: f64, color: Srgba| {
				let pos = rot.transform_vector(vec3(x, y, z) - origin) + origin;
				push_pos(pos.x, pos.y, pos.z, u, v, color);
			};

			let (p0, p1) = {
				let [x0, x1] = minmax(element.from[0], element.to[0]);
				let [y0, y1] = minmax(element.from[1], element.to[1]);
				let [z0, z1] = minmax(element.from[2], element.to[2]);
				(vec3(x0, y0, z0), vec3(x1, y1, z1))
			};
			if let Some(face) = &element.faces.north {
				let (rotate, [u0, v0, u1, v1]) = get_uv(face)?;
				let color = McModelDirection::North.get_shading_srgba();
				if !rotate {
					push_pos(p0.x, p0.y, p0.z, u1, v0, color);
					push_pos(p0.x, p1.y, p0.z, u1, v1, color);
					push_pos(p1.x, p1.y, p0.z, u0, v1, color);
					push_pos(p1.x, p1.y, p0.z, u0, v1, color);
					push_pos(p1.x, p0.y, p0.z, u0, v0, color);
					push_pos(p0.x, p0.y, p0.z, u1, v0, color);
				} else {
					push_pos(p0.x, p0.y, p0.z, u1, v0, color);
					push_pos(p0.x, p1.y, p0.z, u0, v0, color);
					push_pos(p1.x, p1.y, p0.z, u0, v1, color);
					push_pos(p1.x, p1.y, p0.z, u0, v1, color);
					push_pos(p1.x, p0.y, p0.z, u1, v1, color);
					push_pos(p0.x, p0.y, p0.z, u1, v0, color);
				}
			}
			if let Some(face) = &element.faces.east {
				let (rotate, [u0, v0, u1, v1]) = get_uv(face)?;
				let color = McModelDirection::East.get_shading_srgba();
				if !rotate {
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
					push_pos(p1.x, p1.y, p0.z, u1, v1, color);
					push_pos(p1.x, p1.y, p1.z, u0, v1, color);
					push_pos(p1.x, p1.y, p1.z, u0, v1, color);
					push_pos(p1.x, p0.y, p1.z, u0, v0, color);
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
				} else {
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
					push_pos(p1.x, p1.y, p0.z, u0, v0, color);
					push_pos(p1.x, p1.y, p1.z, u0, v1, color);
					push_pos(p1.x, p1.y, p1.z, u0, v1, color);
					push_pos(p1.x, p0.y, p1.z, u1, v1, color);
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
				}
			}
			if let Some(face) = &element.faces.south {
				let (rotate, [u0, v0, u1, v1]) = get_uv(face)?;
				let color = McModelDirection::South.get_shading_srgba();
				if !rotate {
					push_pos(p1.x, p0.y, p1.z, u1, v0, color);
					push_pos(p1.x, p1.y, p1.z, u1, v1, color);
					push_pos(p0.x, p1.y, p1.z, u0, v1, color);
					push_pos(p0.x, p1.y, p1.z, u0, v1, color);
					push_pos(p0.x, p0.y, p1.z, u0, v0, color);
					push_pos(p1.x, p0.y, p1.z, u1, v0, color);
				} else {
					push_pos(p1.x, p0.y, p1.z, u1, v0, color);
					push_pos(p1.x, p1.y, p1.z, u0, v0, color);
					push_pos(p0.x, p1.y, p1.z, u0, v1, color);
					push_pos(p0.x, p1.y, p1.z, u0, v1, color);
					push_pos(p0.x, p0.y, p1.z, u1, v1, color);
					push_pos(p1.x, p0.y, p1.z, u1, v0, color);
				}
			}
			if let Some(face) = &element.faces.west {
				let (rotate, [u0, v0, u1, v1]) = get_uv(face)?;
				let color = McModelDirection::West.get_shading_srgba();
				if !rotate {
					push_pos(p0.x, p0.y, p1.z, u1, v0, color);
					push_pos(p0.x, p1.y, p1.z, u1, v1, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p0.y, p0.z, u0, v0, color);
					push_pos(p0.x, p0.y, p1.z, u1, v0, color);
				} else {
					push_pos(p0.x, p0.y, p1.z, u1, v0, color);
					push_pos(p0.x, p1.y, p1.z, u0, v0, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p0.y, p0.z, u1, v1, color);
					push_pos(p0.x, p0.y, p1.z, u1, v0, color);
				}
			}
			if let Some(face) = &element.faces.up {
				let (rotate, [u0, v0, u1, v1]) = get_uv(face)?;
				let color = McModelDirection::Up.get_shading_srgba();
				if !rotate {
					push_pos(p1.x, p1.y, p1.z, u1, v0, color);
					push_pos(p1.x, p1.y, p0.z, u1, v1, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p1.y, p1.z, u0, v0, color);
					push_pos(p1.x, p1.y, p1.z, u1, v0, color);
				} else {
					push_pos(p1.x, p1.y, p1.z, u1, v0, color);
					push_pos(p1.x, p1.y, p0.z, u0, v0, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p1.y, p0.z, u0, v1, color);
					push_pos(p0.x, p1.y, p1.z, u1, v1, color);
					push_pos(p1.x, p1.y, p1.z, u1, v0, color);
				}
			}
			if let Some(face) = &element.faces.down {
				let (rotate, [u0, v0, u1, v1]) = get_uv(face)?;
				let color = McModelDirection::Down.get_shading_srgba();
				if !rotate {
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
					push_pos(p1.x, p0.y, p1.z, u1, v1, color);
					push_pos(p0.x, p0.y, p1.z, u0, v1, color);
					push_pos(p0.x, p0.y, p1.z, u0, v1, color);
					push_pos(p0.x, p0.y, p0.z, u0, v0, color);
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
				} else {
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
					push_pos(p1.x, p0.y, p1.z, u0, v0, color);
					push_pos(p0.x, p0.y, p1.z, u0, v1, color);
					push_pos(p0.x, p0.y, p1.z, u0, v1, color);
					push_pos(p0.x, p0.y, p0.z, u1, v1, color);
					push_pos(p1.x, p0.y, p0.z, u1, v0, color);
				}
			}
		}
		let mut cpu_mesh = CpuMesh {
			positions: Positions::F64(datas.keys().map(|d| vec3(d.x, d.y, d.z)).collect()),
			indices: match datas.len() {
				0 => Indices::None,
				1..=255 => Indices::U8(indices.into_iter().map(|i| i as u8).collect()),
				256..=65535 => Indices::U16(indices.into_iter().map(|i| i as u16).collect()),
				65536..=4294967295 => Indices::U32(indices.into_iter().collect()), // Wtf huge?
				_ => anyhow::bail!("too many indices: {}", datas.len()),
			},
			normals: None,
			tangents: None,
			uvs: Some(datas.keys().map(|d| vec2(d.u as f32, d.v as f32)).collect()),
			colors: Some(datas.keys().map(|d| d.color).collect()),
		};
		cpu_mesh.compute_normals();
		cpu_mesh.compute_tangents();
		cpu_mesh.compute_aabb();
		Ok((cpu_mesh, texture))
	}
}

fn minmax(a: f64, b: f64) -> [f64; 2] {
	if a < b {
		[a, b]
	} else {
		[b, a]
	}
}
