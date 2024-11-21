
// it's a fun project, but will not benefit me the most

// gotta use some third party crate for tiling
use bevy_ecs_tilemap::prelude::*;
use bevy::{prelude::*, utils::hashbrown::{HashMap, HashSet}};
use rand::Rng;
use bevy_prng::WyRand;
use bevy_rand::prelude::EntropyPlugin;
use bevy_rand::prelude::GlobalEntropy;
use rand_core::RngCore;
#[allow(unused_imports)]
use iter_tools::*;

mod helpers;

fn main() {
  // App::new()
  //   .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
  //   .add_systems(Startup, startup)
  //   .run();
    App::new()
    .add_plugins(DefaultPlugins.set(WindowPlugin{
        primary_window: Some(Window {
            title: String::from(
                "Basic Example - Press Space to change Texture and H to show/hide tilemap.",
            ),
            ..Default::default()
        }),
        ..default()
    }).set(ImagePlugin::default_nearest()))
    .add_plugins(TilemapPlugin)
    .add_plugins(EntropyPlugin::<WyRand>::default())
    .add_systems(Startup, startup)
    .add_systems(Update, collapse_wave_sys)
    .add_systems(Update, helpers::camera::movement)
    // .add_systems(Update, swap_texture_or_hide)
    .run();
}

// // doesn't work because rand::prelude::ThreadRng isn't thread safe
// #[derive(Resource)]
// struct RandomResource(rand::prelude::ThreadRng);


fn startup(
  mut commands: Commands<'_, '_>,
  asset_server: Res<AssetServer>,
  #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {

  commands.spawn(Camera2dBundle::default());
  let texture_handle: Handle<Image> = asset_server.load("terrain_map.png");
  let map_size_factor = 2;
  let map_size = TilemapSize { x:  map_size_factor * 16, y:  map_size_factor * 16 };
  let tilemap_entity = commands.spawn_empty().id();
  let tile_storage = TileStorage::empty(map_size);


  // for x in 0..map_size.x {
  //   for y in 0..map_size.y {
  //     let tile_pos = TilePos {x, y};
  //     let tile_entity = commands.spawn( TileBundle {
  //       position: tile_pos,
  //       tilemap_id: TilemapId(tilemap_entity),
  //       texture_index: TileTextureIndex(random.gen_range(0..4)),
  //       flip: TileFlip {x: random.gen_bool(0.5), ..default()},
  //       ..default()
  //     } ).id();
  //     tile_storage.set(&tile_pos, tile_entity);
  //   }
  // }

  // let tile_size_factor = 2.0;
  // let tile_size = TilemapTileSize {x: tile_size_factor * 16.0, y: tile_size_factor * 16.0};

  // each tile is 16x16 px
  let tile_size = TilemapTileSize {x: 16.0, y: 16.0};

  // let grid_size = tile_size.into();
  // let grid_size = TilemapGridSize { x: 16.0, y: 16.0 };
  let factor = 1.0;
  let grid_size = TilemapGridSize {x: factor * 16.0, y: factor * 16.0};
  let map_type = TilemapType::default();

  commands.entity(tilemap_entity).insert( TilemapBundle {
    grid_size,
    map_type,
    size: map_size,
    storage: tile_storage,
    texture: TilemapTexture::Single(texture_handle),
    tile_size,

    transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0)
      .with_scale(Vec3::splat(3.0)),
    ..default()
  });

  #[cfg(all(not(feature = "atlas"), feature = "render"))]
  {
    array_texture_loader.add(TilemapArrayTexture {
      texture: TilemapTexture::Single(asset_server.load("terrain_map.png")),
      tile_size,
      ..Default::default()
    });
  }
}


fn collapse_wave_sys (
  tilemap_q: Query<&TilemapSize>,
  time: Res<Time>,
  mut timer: Local<f32>,
  mut op_possible_tiles_by_position: Local<Option<
    HashMap<(u32, u32), Vec<u32>>
  >>,
  mut considered_tiles: Local<HashSet<(u32, u32)>>,
  mut grabbed_tiles: Local<HashSet<(u32, u32)>>,
  mut rng: ResMut<GlobalEntropy<WyRand>>,
  mut tile_storage: Query<(Entity, &mut TileStorage)>,
  // mut tilemap_entity: Query<&mut TileStorage>,
  mut commands: Commands,
) {
  let sz = tilemap_q.single();

  // initializing stuff that has yet to be initialized
  if op_possible_tiles_by_position.is_none() {
    info!("possible_tiles_by_position was NONE");
    let tilemap_size = (sz.x, sz.y);
    let possible_values: Vec<u32> = vec![0,1,2,3];
    let mut hash: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    populate(&mut hash, tilemap_size, possible_values);
    *op_possible_tiles_by_position = Some(hash);

    *considered_tiles = HashSet::new();
    // *considered_tiles.insert((sz.x/2, sz.y/2));
    let tile: (u32, u32) = (sz.x/2, sz.y/2);
    (*considered_tiles).insert(tile);
    (*op_possible_tiles_by_position).as_mut().unwrap().get_mut(&tile).unwrap().pop();

    // *local_rng = thread_rng();
  }

  *timer += time.delta_seconds();
  if *timer < 1.0
  { return; }

  *timer = 0.0;
  info!("tilemap size: {}, {}", tilemap_q.single().x, tilemap_q.single().y);
  info!("current considered tiles: {:?}", considered_tiles);

  let (tilemap_entity, mut tile_storage) = tile_storage.single_mut();

  let mut hash = op_possible_tiles_by_position.as_mut().unwrap();
  // select a random tile from the considered ones
  let Some((x, y)) = considered_tiles.iter().nth(rng.next_u32() as usize % considered_tiles.len())
  else {
    warn!("considered tile is empty: ");
    return;
  };
  let Some(possible_values) = hash.get_mut(&(*x, *y))
  else {
    warn!("could not find {} {}", x, y);
    return;
  };
  if possible_values.len() < 1
  {
    warn!("{} {} has no possible values", x, y);
    return;
  }

  // set tile
  let next_tile_index = rng.next_u32() % possible_values.len() as u32;
  let next_tile = possible_values[next_tile_index as usize];
  info!("{} {} -> {}", x, y, next_tile);
  hash.remove(&(*x, *y));
  // hash.entry((*x, *y)).and_modify(|v| v.clear());
  let tile_pos = TilePos {x: *x, y: *y};
  let tile_entity = commands.spawn( TileBundle {
    position: tile_pos,
    tilemap_id: TilemapId(tilemap_entity),
    texture_index: TileTextureIndex(next_tile),
    flip: TileFlip {x: rng.gen_bool(0.5), ..default()},
    ..default()
  }).id();
  grabbed_tiles.insert((*x, *y));
  tile_storage.set(&tile_pos, tile_entity);

  // modify what values neighbouring tiles can take
  let mut neighbours: Vec<(u32, u32)> = get_neighbours(*x, *y, sz.x, sz.y);
  // let debug_values = hash.iter()
  //   .filter(|(k, _)| neighbours.contains(k))
  //   .collect_vec();
  // println!("Possible neighbour values before altering: {:?}", debug_values);
  alter_neighbour_valid_tiles(
    &mut neighbours,
    &mut hash,
    next_tile
  );

  // update considered tiles
  (*considered_tiles).clear();
  let mut possibilities_by_neighbour_tiles: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
  
  for (x, y) in grabbed_tiles.iter() {
    let neighbours = get_neighbours(*x, *y, sz.x, sz.y);
    for (neigh_x, neigh_y) in neighbours.iter() {
      let Some(v) = hash.get(&(*neigh_x, *neigh_y))
      else { continue; };
      possibilities_by_neighbour_tiles.insert((*neigh_x, *neigh_y), v.clone());
    }
  }
  // fill_considered_tiles(&hash, &mut considered_tiles);
  fill_considered_tiles(&possibilities_by_neighbour_tiles, &mut considered_tiles);
  // clear_considered_tiles_from_hash(&mut hash, &considered_tiles);
  info!("Refilled considered tiles: {:?}", considered_tiles);


}

const VALID_COMBOS: &[&[u32]] = &[
  &[0, 1, 2, 3], 
  &[0, 1, 2],    
  &[1, 2, 3],    
  &[1, 2, 3],  
];

const DEFAULT_COMBO: &[u32] = &[0, 1, 2, 3];


//---------//
// UTILITY //
//---------//

fn populate(
  possible_tiles_by_position: &mut HashMap<(u32, u32), Vec<u32>>,
  tilemap_size: (u32, u32),
  possible_values: Vec<u32>,
) {
  for x in 0..tilemap_size.0 { for y in 0..tilemap_size.1 {
    possible_tiles_by_position.entry((x, y))
      .or_insert(possible_values.clone());
  }}
  // hash
}

fn get_neighbours(
  x: u32,
  y: u32,
  x_max: u32,
  y_max: u32
) -> Vec<(u32, u32)> {
  let mut output: Vec<(u32, u32)> = Vec::new();
  if x > 0 { output.push((x-1, y)); }
  if x < x_max { output.push((x+1, y)); }
  if y > 0 { output.push((x, y-1)); }
  if y < y_max { output.push((x, y+1)); }
  output
}

fn alter_neighbour_valid_tiles(
  neighbours: &mut Vec<(u32, u32)>,
  possible_tiles_by_position: &mut HashMap<(u32, u32), Vec<u32>>,
  next_tile: u32,
) {
  for (x, y) in neighbours.iter() {
    let Some(current_possible_values) = possible_tiles_by_position.get(&(*x, *y)).cloned()
    else { continue; };

    let next_valid_combo = VALID_COMBOS.get(next_tile as usize)
      .unwrap_or(&DEFAULT_COMBO);
    let next_valid_combo: HashSet<u32> = HashSet::from_iter(next_valid_combo.iter().cloned());
    let current_possible_values: HashSet<u32> = HashSet::from_iter(current_possible_values.iter().cloned());
    let join_of_possibilities = next_valid_combo.intersection(&current_possible_values);
    println!("next valid combo {:?}, current possible values: {:?}, join: {:?}, next tile: {}", next_valid_combo, current_possible_values, join_of_possibilities, next_tile);
    possible_tiles_by_position.entry((*x, *y))
      .and_modify(|v|
        *v = join_of_possibilities
          .map(|x| *x)
          .collect()
      );
  }
}

fn fill_considered_tiles(neighbours: &HashMap<(u32, u32), Vec<u32>>, considered_tiles: &mut HashSet<(u32, u32)>) {
  let Some(min_len) = neighbours.values().map(Vec::len).min()
  else { return; };
  let least_options_tiles: Vec<(u32, u32)> = neighbours.iter()
    .filter(|(_, v)| v.len() == min_len)
    .map(|(k, _)| *k)
    .collect();
  considered_tiles.extend(least_options_tiles);
}

#[deprecated(note="should remove immediately upon assigning a value to a tile")]
#[allow(unused)]
fn clear_considered_tiles_from_hash(hash: &mut HashMap<(u32, u32), Vec<u32>>, considered_tiles: &HashSet<(u32, u32)>) {
  hash.retain(|k, _| considered_tiles.contains(k));
}