// Cross-platform IDCT parity check.
//
// Compares `aan_test_deterministic_bytes` (active SIMD dispatch) against
// `aan_test_deterministic_bytes_scalar` (forced scalar) on the same
// 12-block deterministic fixture.  On a correct build the two byte
// vectors must be identical.  Any divergence means the active SIMD path
// (NEON on aarch64, AVX2 on x86_64) disagrees with the scalar reference.
//
// Known result (phasm-core 0.3.2, x86_64+AVX2):
//   `idct_kernel_avx2` has an out-of-bounds store bug and incorrect
//   inter-pass register packing; every IDCT block differs from scalar.
//
// Usage:
//   # native
//   cargo run --example idct_parity_check
//
//   # cross-architecture proof from aarch64 → x86_64 via Miri
//   RUSTFLAGS="-C target-feature=+avx2" \
//     cargo +nightly miri run --target x86_64-unknown-linux-gnu \
//       --example idct_parity_check

use phasm_core::codec::jpeg::pixels_aan::{
    aan_test_deterministic_bytes, aan_test_deterministic_bytes_scalar,
};

fn simd_flags() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        let sse42 = std::arch::is_x86_feature_detected!("sse4.2");
        let avx   = std::arch::is_x86_feature_detected!("avx");
        let avx2  = std::arch::is_x86_feature_detected!("avx2");
        return format!("sse4.2={sse42}  avx={avx}  avx2={avx2}");
    }
    #[cfg(target_arch = "aarch64")]
    {
        let neon = std::arch::is_aarch64_feature_detected!("neon");
        let sve  = std::arch::is_aarch64_feature_detected!("sve");
        return format!("neon={neon}  sve={sve}");
    }
    #[allow(unreachable_code)]
    String::from("(no feature-detection available on this arch)")
}

fn main() {
    println!("os     : {}", std::env::consts::OS);
    println!("arch   : {}", std::env::consts::ARCH);
    println!("flags  : {}", simd_flags());
    println!("active : aan_test_deterministic_bytes        (SIMD dispatch)");
    println!("oracle : aan_test_deterministic_bytes_scalar (forced scalar)");
    println!();

    let active = aan_test_deterministic_bytes();
    let scalar = aan_test_deterministic_bytes_scalar();

    // Each 12-block fixture entry is 64×2 (i16 coeffs) + 64×4 (i32 pixels) = 384 bytes.
    let entry_bytes = 64 * 2 + 64 * 4;
    let n_blocks = active.len() / entry_bytes;

    let mut all_pass = true;
    for b in 0..n_blocks {
        let start = b * entry_bytes;
        let end   = start + entry_bytes;
        let a_entry = &active[start..end];
        let s_entry = &scalar[start..end];

        if a_entry == s_entry {
            println!("block {b:2}: PASS");
        } else {
            all_pass = false;
            let first_diff = a_entry.iter().zip(s_entry).position(|(a, s)| a != s).unwrap();
            println!(
                "block {b:2}: FAIL — first byte mismatch at offset {first_diff} \
                 (active=0x{:02x} scalar=0x{:02x})",
                a_entry[first_diff], s_entry[first_diff],
            );
        }
    }

    println!();
    if all_pass {
        println!("result : PASS — active SIMD path is byte-identical to scalar");
        std::process::exit(0);
    } else {
        println!("result : FAIL — active SIMD path diverges from scalar reference");
        std::process::exit(1);
    }
}
