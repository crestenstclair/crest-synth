//! Preserve STK delay history and mesh equations while reducing memory traffic.
use std::path::{Path, PathBuf};

pub fn stage(output: &Path) -> PathBuf {
    let destination = output.join("stk-source");
    let mut header = std::fs::read_to_string("vendor/audio/stk/include/BandedWG.h").unwrap();
    replace(
        &mut header,
        "#include \"DelayL.h\"",
        "#include \"banded_delay.h\"",
    );
    replace(&mut header, "DelayL   delay_", "BandedDelay delay_");
    write(&destination.join("BandedWG.h"), &header);
    write(
        &destination.join("BandedWG.cpp"),
        &std::fs::read_to_string("vendor/audio/stk/src/BandedWG.cpp").unwrap(),
    );

    let mut mesh = std::fs::read_to_string("vendor/audio/stk/src/Mesh2D.cpp").unwrap();
    for (tick, suffix) in [("tick0", ""), ("tick1", "1")] {
        let function = mesh
            .find(&format!("StkFloat Mesh2D :: {tick}( void )"))
            .unwrap();
        let start = function
            + mesh[function..]
                .find("  // Update junction velocities.")
                .unwrap();
        let end = start
            + mesh[start..]
                .find("      // Update positive-going waves.")
                .unwrap();
        // The outgoing pass reads only this junction's velocity and the
        // original buffers. Its writes all target the alternate buffers.
        let fused = format!(
            r"  // Update each junction and its outgoing alternate-buffer waves.
  for (x=0; x<NX_-1; x++) {{
    for (y=0; y<NY_-1; y++) {{
      StkFloat vxy = ( vxp{suffix}_[x][y] + vxm{suffix}_[x+1][y] +
                      vyp{suffix}_[x][y] + vym{suffix}_[x][y+1] ) * VSCALE;
      v_[x][y] = vxy;
"
        );
        mesh.replace_range(start..end, &fused);
    }
    write(&destination.join("Mesh2D.cpp"), &mesh);
    destination
}

fn replace(text: &mut String, before: &str, after: &str) {
    assert_eq!(
        text.matches(before).count(),
        1,
        "STK adaptation no longer matches: {before}"
    );
    *text = text.replace(before, after);
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    if std::fs::read_to_string(path).ok().as_deref() != Some(text) {
        std::fs::write(path, text).unwrap();
    }
}
