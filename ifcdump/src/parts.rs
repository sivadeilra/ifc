use super::*;

pub fn dump_parts(ifc: &Ifc) -> Result<()> {
    let mut parts_sorted: Vec<_> = ifc.parts().iter().collect();
    parts_sorted.sort_unstable_by_key(|&(k, _)| k);

    println!("Partitions:");
    for (part_name, part_entry) in parts_sorted.iter() {
        println!(
            "{:-40}     entry size: {:3}, num_entries: {}",
            part_name, part_entry.size, part_entry.count
        );
    }
    println!();

    Ok(())
}
