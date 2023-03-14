use bytes::{BufMut, Bytes, BytesMut};
use util::{Deserialize, Serialize, Vector, Vector3b};

use crate::{biome::Biome3d, database::RawDatabase, DatabaseKey, Dimension, KeyData, SubChunk, BIOME_DATA, LOCAL_PLAYER, MOB_EVENTS, OVERWORLD, SCHEDULER, SCOREBOARD, LevelData};

// digp [x] [z] [?dimension]
// contains two int32
// can pack multiple into one
// points to "actorprefix" + digp data

// palette: [Compound({"states": Compound({"pillar_axis": String("y")}), "version": Int(17959425), "name": String("minecraft:deepslate")}), Compound({"states": Compound({"stone_type": String("stone")}), "version": Int(17959425), "name": String("minecraft:stone")}), Compound({"states": Compound({}), "name": String("minecraft:iron_ore"), "version": Int(17959425)}), Compound({"name": String("minecraft:gravel"), "states": Compound({}), "version": Int(17959425)}), Compound({"states": Compound({}), "name": String("minecraft:deepslate_iron_ore"), "version": Int(17959425)}), Compound({"states": Compound({"stone_type": String("diorite")}), "version": Int(17959425), "name": String("minecraft:stone")}), Compound({"name": String("minecraft:dirt"), "states": Compound({"dirt_type": String("normal")}), "version": Int(17959425)}), Compound({"states": Compound({}), "version": Int(17959425), "name": String("minecraft:deepslate_redstone_ore")}), Compound({"version": Int(17959425), "states": Compound({}), "name": String("minecraft:deepslate_copper_ore")}), Compound({"name": String("minecraft:copper_ore"), "version": Int(17959425), "states": Compound({})}), Compound({"states": Compound({}), "name": String("minecraft:deepslate_lapis_ore"), "version": Int(17959425)}), Compound({"version": Int(17959425), "name": String("minecraft:stone"), "states": Compound({"stone_type": String("granite")})}), Compound({"states": Compound({}), "version": Int(17959425), "name": String("minecraft:lapis_ore")}), Compound({"version": Int(17959425), "name": String("minecraft:redstone_ore"), "states": Compound({})}), Compound({"version": Int(17959425), "states": Compound({"stone_type": String("andesite")}), "name": String("minecraft:stone")}), Compound({"version": Int(17959425), "name": String("minecraft:air"), "states": Compound({})})] }]
#[ignore]
#[test]
fn database_test() {
    let db = RawDatabase::new("test/db").unwrap();
    let mut iter = db.iter();

    // let mut count = 0;
    // let mut sum = 0;

    for raw_ref in iter {
        let key = raw_ref.key();
        if key[key.len() - 2] == 0x2f {
            // count += 1;

            // let instant = std::time::Instant::now();
            let subchunk = SubChunk::deserialize(raw_ref.value().as_ref()).unwrap();
            // let elapsed = instant.elapsed();
            //
            // sum += elapsed.as_micros();
            //
            // std::hint::black_box(subchunk);
            // println!("elapsed: {elapsed:?}");
        }
    }

    // let avg = sum as f64 / count as f64;
    // println!("average: {avg}µs");

    // let mut buffer = BytesMut::new();
    // DatabaseKey {
    //     x: -2,
    //     z: -1,
    //     data: KeyData::SubChunk {
    //         index: 6
    //     },
    //     dimension: Dimension::Overworld
    // }.serialize(&mut buffer);
    //
    // let data = db.get_raw_key(buffer).unwrap();
    // let subchunk = SubChunk::deserialize(data).unwrap();
    // let block = subchunk.get(Vector::from([0, 0, 0])).unwrap();
    // dbg!(block);
}

#[test]
fn load_level_dat() {
    const LEVEL_DAT: &[u8] = include_bytes!("../test/level.dat");

    let decoded: LevelData = nbt::from_le_bytes(&LEVEL_DAT[8..]).unwrap().0;
    dbg!(decoded);
}