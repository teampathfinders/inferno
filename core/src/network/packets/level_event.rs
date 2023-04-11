use util::{bail, Deserialize, Error, Result, Serialize, Vector};
use util::bytes::{BinaryRead, BinaryWrite, MutableBuffer, SharedBuffer, size_of_varint};

use crate::network::ConnectedPacket;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LevelEventType {
    SoundClick = 1000,
    SoundClickFail = 1001,
    SoundLaunch = 1002,
    SoundOpenDoor = 1003,
    SoundFizz = 1004,
    SoundFuse = 1005,
    SoundPlayRecording = 1006,
    SoundGhastWarning = 1007,
    SoundGhastFireball = 1008,
    SoundBlazeFireball = 1009,
    SoundZombieWoodenDoor = 1010,
    SoundZombieDoorCrash = 1012,
    SoundZombieInfected = 1016,
    SoundZombieConverted = 1017,
    SoundEndermanTeleport = 1018,
    SoundAnvilBroken = 1020,
    SoundAnvilUsed = 1021,
    SoundAnvilLand = 1022,
    SoundInfinityArrowPickup = 1030,
    SoundTeleportEnderPearl = 1032,
    SoundAddItem = 1040,
    SoundItemFrameBreak = 1041,
    SoundItemFramePlace = 1042,
    SoundItemFrameRemoveItem = 1043,
    SoundItemFrameRotateItem = 1044,
    SoundExperienceOrbPickup = 1051,
    SoundTotemUsed = 1052,
    SoundArmorStandBreak = 1060,
    SoundArmorStandHit = 1061,
    SoundArmorStandLand = 1062,
    SoundArmorStandPlace = 1063,
    SoundPointedDripstoneLand = 1064,
    SoundDyeUsed = 1065,
    SoundInkSacUsed = 1066,
    QueueCustomMusic = 1900,
    PlayCustomMusic = 1901,
    StopCustomMusic = 1902,
    SetMusicVolume = 1903,
    ParticlesShoot = 2000,
    ParticlesDestroyBlock = 2001,
    ParticlesPotionSplash = 2002,
    ParticlesEyeOfEnderDeath = 2003,
    ParticlesMobBlockSpawn = 2004,
    ParticlesCropGrowth = 2005,
    ParticlesSoundGuardianGhost = 2006,
    ParticlesDeathSmoke = 2007,
    ParticlesDenyBlock = 2008,
    ParticlesGenericSpawn = 2009,
    ParticlesDragonEgg = 2010,
    ParticlesCropEaten = 2011,
    ParticlesCritical = 2012,
    ParticlesTeleport = 2013,
    ParticlesCrackBlock = 2014,
    ParticlesBubble = 2015,
    ParticlesEvaporate = 2016,
    ParticlesDestroyArmorStand = 2017,
    ParticlesBreakingEgg = 2018,
    ParticlesDestroyEgg = 2019,
    ParticlesEvaporateWater = 2020,
    ParticlesDestroyBlockNoSound = 2021,
    ParticlesKnockbackRoar = 2022,
    ParticlesTeleportTrail = 2023,
    ParticlesPointCloud = 2024,
    ParticlesExplosion = 2025,
    ParticlesBlockExplosion = 2026,
    ParticlesVibrationSignal = 2027,
    ParticlesDripstoneDrip = 2028,
    ParticlesFizzEffect = 2029,
    WaxOn = 2030,
    WaxOff = 2031,
    Scrape = 2032,
    ParticlesElectricSpark = 2033,
    ParticlesTurtleEgg = 2034,
    ParticlesSculkShriek = 2035,
    SculkCatalystBloom = 2036,
    SculkCharge = 2037,
    SculkChargePop = 2038,
    SonicExplosion = 2039,
    StartRaining = 3001,
    StartThunderstorm = 3002,
    StopRaining = 3003,
    StopThunderstorm = 3004,
    GlobalPause = 3005,
    SimTimeStep = 3006,
    SimTimeScale = 3007,
    ActivateBlock = 3500,
    CauldronExplode = 3501,
    CauldronDyeArmor = 3502,
    CauldronCleanArmor = 3503,
    CauldronFillPotion = 3504,
    CauldronTakePotion = 3505,
    CauldronFillWater = 3506,
    CauldronTakeWater = 3507,
    CauldronAddDye = 3508,
    CauldronCleanBanner = 3509,
    CauldronFlush = 3510,
    AgentSpawnEffect = 3511,
    CauldronFillLava = 3512,
    CauldronTakeLava = 3513,
    CauldronFillPowderSnow = 3514,
    CauldronTakePowderSnow = 3515,
    StartBlockCracking = 3600,
    StopBlockCracking = 3601,
    UpdateBlockCracking = 3602,
    AllPlayersSleeping = 9800,
    SleepingPlayers = 9801,
    JumpPrevented = 9810,
    ParticlesLegacyEvent = 0x4000,
}

impl TryFrom<i32> for LevelEventType {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> anyhow::Result<Self> {
        Ok(match value {
            1000 => Self::SoundClick,
            1001 => Self::SoundClickFail,
            1002 => Self::SoundLaunch,
            1003 => Self::SoundOpenDoor,
            1004 => Self::SoundFizz,
            1005 => Self::SoundFuse,
            1006 => Self::SoundPlayRecording,
            1007 => Self::SoundGhastWarning,
            1008 => Self::SoundGhastFireball,
            1009 => Self::SoundBlazeFireball,
            1010 => Self::SoundZombieWoodenDoor,
            1012 => Self::SoundZombieDoorCrash,
            1016 => Self::SoundZombieInfected,
            1017 => Self::SoundZombieConverted,
            1018 => Self::SoundEndermanTeleport,
            1020 => Self::SoundAnvilBroken,
            1021 => Self::SoundAnvilUsed,
            1022 => Self::SoundAnvilLand,
            1030 => Self::SoundInfinityArrowPickup,
            1032 => Self::SoundTeleportEnderPearl,
            1040 => Self::SoundAddItem,
            1041 => Self::SoundItemFrameBreak,
            1042 => Self::SoundItemFramePlace,
            1043 => Self::SoundItemFrameRemoveItem,
            1044 => Self::SoundItemFrameRotateItem,
            1051 => Self::SoundExperienceOrbPickup,
            1052 => Self::SoundTotemUsed,
            1060 => Self::SoundArmorStandBreak,
            1061 => Self::SoundArmorStandHit,
            1062 => Self::SoundArmorStandLand,
            1063 => Self::SoundArmorStandPlace,
            1064 => Self::SoundPointedDripstoneLand,
            1065 => Self::SoundDyeUsed,
            1066 => Self::SoundInkSacUsed,
            1900 => Self::QueueCustomMusic,
            1901 => Self::PlayCustomMusic,
            1902 => Self::StopCustomMusic,
            1903 => Self::SetMusicVolume,
            2000 => Self::ParticlesShoot,
            2001 => Self::ParticlesDestroyBlock,
            2002 => Self::ParticlesPotionSplash,
            2003 => Self::ParticlesEyeOfEnderDeath,
            2004 => Self::ParticlesMobBlockSpawn,
            2005 => Self::ParticlesCropGrowth,
            2006 => Self::ParticlesSoundGuardianGhost,
            2007 => Self::ParticlesDeathSmoke,
            2008 => Self::ParticlesDenyBlock,
            2009 => Self::ParticlesGenericSpawn,
            2010 => Self::ParticlesDragonEgg,
            2011 => Self::ParticlesCropEaten,
            2012 => Self::ParticlesCritical,
            2013 => Self::ParticlesTeleport,
            2014 => Self::ParticlesCrackBlock,
            2015 => Self::ParticlesBubble,
            2016 => Self::ParticlesEvaporate,
            2017 => Self::ParticlesDestroyArmorStand,
            2018 => Self::ParticlesBreakingEgg,
            2019 => Self::ParticlesDestroyEgg,
            2020 => Self::ParticlesEvaporateWater,
            2021 => Self::ParticlesDestroyBlockNoSound,
            2022 => Self::ParticlesKnockbackRoar,
            2023 => Self::ParticlesTeleportTrail,
            2024 => Self::ParticlesPointCloud,
            2025 => Self::ParticlesExplosion,
            2026 => Self::ParticlesBlockExplosion,
            2027 => Self::ParticlesVibrationSignal,
            2028 => Self::ParticlesDripstoneDrip,
            2029 => Self::ParticlesFizzEffect,
            2030 => Self::WaxOn,
            2031 => Self::WaxOff,
            2032 => Self::Scrape,
            2033 => Self::ParticlesElectricSpark,
            2034 => Self::ParticlesTurtleEgg,
            2035 => Self::ParticlesSculkShriek,
            2036 => Self::SculkCatalystBloom,
            2037 => Self::SculkCharge,
            2038 => Self::SculkChargePop,
            2039 => Self::SonicExplosion,
            3001 => Self::StartRaining,
            3002 => Self::StartThunderstorm,
            3003 => Self::StopRaining,
            3004 => Self::StopThunderstorm,
            3005 => Self::GlobalPause,
            3006 => Self::SimTimeStep,
            3007 => Self::SimTimeScale,
            3500 => Self::ActivateBlock,
            3501 => Self::CauldronExplode,
            3502 => Self::CauldronDyeArmor,
            3503 => Self::CauldronCleanArmor,
            3504 => Self::CauldronFillPotion,
            3505 => Self::CauldronTakePotion,
            3506 => Self::CauldronFillWater,
            3507 => Self::CauldronTakeWater,
            3508 => Self::CauldronAddDye,
            3509 => Self::CauldronCleanBanner,
            3510 => Self::CauldronFlush,
            3511 => Self::AgentSpawnEffect,
            3512 => Self::CauldronFillLava,
            3513 => Self::CauldronTakeLava,
            3514 => Self::CauldronFillPowderSnow,
            3515 => Self::CauldronTakePowderSnow,
            3600 => Self::StartBlockCracking,
            3601 => Self::StopBlockCracking,
            3602 => Self::UpdateBlockCracking,
            9800 => Self::AllPlayersSleeping,
            9801 => Self::SleepingPlayers,
            9810 => Self::JumpPrevented,
            0x4000 => Self::ParticlesLegacyEvent,
            _ => bail!(Malformed, "Invalid level event type {value}")
        })
    }
}

#[derive(Debug, Clone)]
pub struct LevelEvent {
    pub event_type: LevelEventType,
    pub position: Vector<f32, 3>,
    pub event_data: i32,
}

impl ConnectedPacket for LevelEvent {
    const ID: u32 = 0x19;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.event_type as i32) + 3 * 4 +
            size_of_varint(self.event_data)
    }
}

impl Serialize for LevelEvent {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()> where W: BinaryWrite {
        buffer.write_var_i32(self.event_type as i32)?;
        buffer.write_vecf(&self.position)?;
        buffer.write_var_i32(self.event_data)
    }
}

impl Deserialize<'_> for LevelEvent {
    fn deserialize(mut buffer: SharedBuffer) -> anyhow::Result<Self> {
        let event_type = LevelEventType::try_from(buffer.read_var_i32()?)?;
        let position = buffer.read_vecf()?;
        let event_data = buffer.read_var_i32()?;

        Ok(Self {
            event_type,
            position,
            event_data,
        })
    }
}