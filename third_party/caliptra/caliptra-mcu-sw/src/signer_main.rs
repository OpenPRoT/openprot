// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use caliptra_auth_man_gen::{
    AuthManifestGenerator, AuthManifestGeneratorConfig, AuthManifestGeneratorKeyConfig,
};
use caliptra_auth_man_types::{
    Addr64, AuthManifestFlags, AuthManifestImageMetadata, AuthManifestPrivKeysConfig,
    AuthManifestPubKeysConfig, ImageMetadataFlags,
};
use caliptra_image_crypto::RustCrypto as Crypto;
use caliptra_image_fake_keys::*;
use caliptra_image_gen::{from_hw_format, ImageGeneratorCrypto};
use caliptra_image_types::FwVerificationPqcKeyType;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zerocopy::IntoBytes;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Signer {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Original firmware-bundler commands
    #[command(subcommand)]
    Experimental(firmware_bundler::args::Commands),

    /// Auth Manifest generation
    AuthManifest {
        #[command(subcommand)]
        subcommand: AuthManifestCommands,
    },
}

#[derive(Subcommand)]
enum AuthManifestCommands {
    /// Create an Authentication Manifest (SoC manifest)
    Create {
        /// MCU Image: <path>,<load_addr>,<staging_addr>,<image_id>,<exec_bit>
        #[arg(long = "mcu_image")]
        mcu_image: String,

        /// Path to caliptra ROM binary (accepted for compatibility, not used in manifest)
        #[arg(long = "caliptra_rom")]
        caliptra_rom: Option<PathBuf>,

        /// Path to caliptra firmware binary (accepted for compatibility, not used in manifest)
        #[arg(long = "caliptra_firmware")]
        caliptra_firmware: Option<PathBuf>,

        /// SHA384 of vendor public key (accepted for compatibility, not used in manifest)
        #[arg(long = "vendor_pk_hash")]
        vendor_pk_hash: Option<String>,

        /// Output file path
        #[arg(long = "output")]
        output: String,
    },
}

/// Parse an image config string: "path,load_addr,staging_addr,image_id,exec_bit[,component_id]"
fn parse_image_cfg(s: &str) -> Result<(PathBuf, u64, u64, u32, u32, u32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() < 5 {
        bail!(
            "Expected at least 5 comma-separated fields (path,load_addr,staging_addr,image_id,exec_bit), got {}",
            parts.len()
        );
    }
    let path = PathBuf::from(parts[0]);
    let load_addr = parse_addr(parts[1])?;
    let staging_addr = parse_addr(parts[2])?;
    let image_id = parts[3].parse::<u32>()?;
    let exec_bit = parts[4].parse::<u32>()?;
    let component_id = if parts.len() > 5 {
        parts[5].parse::<u32>()?
    } else {
        0
    };
    Ok((path, load_addr, staging_addr, image_id, exec_bit, component_id))
}

fn parse_addr(s: &str) -> Result<u64> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Ok(u64::from_str_radix(hex, 16)?)
    } else {
        Ok(s.parse::<u64>()?)
    }
}

fn create_image_metadata(
    image_path: &PathBuf,
    load_addr: u64,
    staging_addr: u64,
    image_id: u32,
    exec_bit: u32,
    component_id: u32,
) -> Result<AuthManifestImageMetadata> {
    const IMAGE_SOURCE_IN_REQUEST: u32 = 1;

    let data = std::fs::read(image_path)?;
    let mut flags = ImageMetadataFlags(0);
    flags.set_ignore_auth_check(false);
    flags.set_image_source(IMAGE_SOURCE_IN_REQUEST);
    flags.set_exec_bit(exec_bit);

    let crypto = Crypto::default();
    let digest = from_hw_format(&crypto.sha384_digest(&data)?);

    Ok(AuthManifestImageMetadata {
        fw_id: image_id,
        flags: flags.0,
        component_id,
        digest,
        image_staging_address: Addr64 {
            lo: staging_addr as u32,
            hi: (staging_addr >> 32) as u32,
        },
        image_load_address: Addr64 {
            lo: load_addr as u32,
            hi: (load_addr >> 32) as u32,
        },
        ..Default::default()
    })
}

fn create_auth_manifest(
    image_metadata_list: Vec<AuthManifestImageMetadata>,
) -> Result<Vec<u8>> {
    let vendor_fw_key_info = AuthManifestGeneratorKeyConfig {
        pub_keys: AuthManifestPubKeysConfig {
            ecc_pub_key: VENDOR_ECC_KEY_0_PUBLIC,
            lms_pub_key: VENDOR_LMS_KEY_0_PUBLIC,
            mldsa_pub_key: VENDOR_MLDSA_KEY_0_PUBLIC,
        },
        priv_keys: Some(AuthManifestPrivKeysConfig {
            ecc_priv_key: VENDOR_ECC_KEY_0_PRIVATE,
            lms_priv_key: VENDOR_LMS_KEY_0_PRIVATE,
            mldsa_priv_key: VENDOR_MLDSA_KEY_0_PRIVATE,
        }),
    };

    let vendor_man_key_info = AuthManifestGeneratorKeyConfig {
        pub_keys: AuthManifestPubKeysConfig {
            ecc_pub_key: VENDOR_ECC_KEY_1_PUBLIC,
            lms_pub_key: VENDOR_LMS_KEY_1_PUBLIC,
            mldsa_pub_key: VENDOR_MLDSA_KEY_0_PUBLIC,
        },
        priv_keys: Some(AuthManifestPrivKeysConfig {
            ecc_priv_key: VENDOR_ECC_KEY_1_PRIVATE,
            lms_priv_key: VENDOR_LMS_KEY_1_PRIVATE,
            mldsa_priv_key: VENDOR_MLDSA_KEY_0_PRIVATE,
        }),
    };

    let owner_fw_key_info = Some(AuthManifestGeneratorKeyConfig {
        pub_keys: AuthManifestPubKeysConfig {
            ecc_pub_key: OWNER_ECC_KEY_PUBLIC,
            lms_pub_key: OWNER_LMS_KEY_PUBLIC,
            mldsa_pub_key: OWNER_MLDSA_KEY_PUBLIC,
        },
        priv_keys: Some(AuthManifestPrivKeysConfig {
            ecc_priv_key: OWNER_ECC_KEY_PRIVATE,
            lms_priv_key: OWNER_LMS_KEY_PRIVATE,
            mldsa_priv_key: OWNER_MLDSA_KEY_PRIVATE,
        }),
    });

    let owner_man_key_info = Some(AuthManifestGeneratorKeyConfig {
        pub_keys: AuthManifestPubKeysConfig {
            ecc_pub_key: OWNER_ECC_KEY_PUBLIC,
            lms_pub_key: OWNER_LMS_KEY_PUBLIC,
            mldsa_pub_key: OWNER_MLDSA_KEY_PUBLIC,
        },
        priv_keys: Some(AuthManifestPrivKeysConfig {
            ecc_priv_key: OWNER_ECC_KEY_PRIVATE,
            lms_priv_key: OWNER_LMS_KEY_PRIVATE,
            mldsa_priv_key: OWNER_MLDSA_KEY_PRIVATE,
        }),
    });

    let gen_config = AuthManifestGeneratorConfig {
        vendor_fw_key_info: Some(vendor_fw_key_info),
        vendor_man_key_info: Some(vendor_man_key_info),
        owner_fw_key_info,
        owner_man_key_info,
        image_metadata_list,
        version: 1,
        flags: AuthManifestFlags::VENDOR_SIGNATURE_REQUIRED,
        pqc_key_type: FwVerificationPqcKeyType::LMS,
        svn: 0,
    };

    let gen = AuthManifestGenerator::new(Crypto::default());
    let manifest = gen.generate(&gen_config)?;
    Ok(manifest.as_bytes().to_vec())
}

fn main() -> Result<()> {
    let args = Signer::parse();
    match args.cmd {
        Commands::Experimental(cmd) => firmware_bundler::execute(cmd),
        Commands::AuthManifest { subcommand } => match subcommand {
            AuthManifestCommands::Create {
                mcu_image,
                output,
                ..
            } => {
                let (path, load_addr, staging_addr, image_id, exec_bit, component_id) =
                    parse_image_cfg(&mcu_image)?;

                let metadata = create_image_metadata(
                    &path,
                    load_addr,
                    staging_addr,
                    image_id,
                    exec_bit,
                    component_id,
                )?;

                let manifest_bytes = create_auth_manifest(vec![metadata])?;
                std::fs::write(&output, &manifest_bytes)?;
                Ok(())
            }
        },
    }
}
