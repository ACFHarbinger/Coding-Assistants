use super::*;

#[test]
fn parses_a_two_gpu_csv_with_byte_math() {
    let gpus = parse_gpu_csv(
        "0, NVIDIA GeForce RTX 4070, 12, 1024, 12282\n1, NVIDIA GeForce RTX 4070, 0, 512, 12282\n",
    );
    assert_eq!(gpus.len(), 2);
    assert_eq!(gpus[0].index, 0);
    assert_eq!(gpus[0].name, "NVIDIA GeForce RTX 4070");
    assert!((gpus[0].usage_percent - 12.0).abs() < f32::EPSILON);
    assert_eq!(gpus[0].mem_used_bytes, 1024 * MIB);
    assert_eq!(gpus[0].mem_total_bytes, 12282 * MIB);
    assert_eq!(gpus[1].index, 1);
}

#[test]
fn gpu_names_containing_commas_survive() {
    // The name is everything between the index and the last three numeric
    // readings — a comma inside it must not shift the columns.
    let gpus = parse_gpu_csv("0, Some, Fancy GPU XL, 42, 100, 200\n");
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].name, "Some, Fancy GPU XL");
    assert_eq!(gpus[0].mem_used_bytes, 100 * MIB);
    assert_eq!(gpus[0].mem_total_bytes, 200 * MIB);
}

#[test]
fn malformed_gpu_lines_are_skipped_not_fatal() {
    let gpus = parse_gpu_csv(
        "\n   \n0, Good GPU, 5, 10, 100\nshort, line\n1, Bad Percent, 101, 10, 100\n2, , 5, 10, 100\n",
    );
    assert_eq!(gpus.len(), 1);
    assert_eq!(gpus[0].name, "Good GPU");
}

#[test]
fn mib_conversion_rejects_garbage() {
    assert_eq!(mib_to_bytes(1.0), MIB);
    assert_eq!(mib_to_bytes(0.0), 0);
    assert_eq!(mib_to_bytes(-3.0), 0);
    assert_eq!(mib_to_bytes(f32::NAN), 0);
    assert_eq!(mib_to_bytes(f32::INFINITY), 0);
}

#[test]
fn snapshot_reads_sane_values_on_this_host() {
    // Runs anywhere CI runs: asserts shape, not particular readings.
    let snapshot = collect_snapshot().unwrap();
    assert!(snapshot.fetched_at > 0);
    assert!(!snapshot.cpus.is_empty());
    assert!(snapshot.memory.total_bytes > 0);
    assert!(snapshot.memory.used_bytes <= snapshot.memory.total_bytes);
    assert!(!snapshot.disks.is_empty());
    assert!(
        snapshot.disks.iter().all(|disk| disk.total_bytes > 0),
        "zero-total pseudo-filesystems must be filtered"
    );
    // GPU is best-effort: either populated or honestly unavailable.
    if snapshot.gpu.available {
        assert!(!snapshot.gpu.gpus.is_empty());
        assert!(snapshot.gpu.detail.is_none());
    } else {
        assert!(snapshot
            .gpu
            .detail
            .as_deref()
            .is_some_and(|d| !d.is_empty()));
    }
}
