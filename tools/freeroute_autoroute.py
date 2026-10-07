(kicad_pcb
  (version 20260206)
  (generator "synth-eda")
  (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (44 "Edge.Cuts" user))
  (net 0 "")
  (net 1 "SIG")
  (footprint "Resistor_SMD:R_0603_1608Metric"
    (layer "F.Cu") (at 10 20)
    (property "Reference" "R1" (at 0 -1) (layer "F.SilkS"))
    (pad "1" smd roundrect (at -0.825 0) (size 0.8 0.95)
      (layers "F.Cu" "F.Paste" "F.Mask") (net 1 "SIG")))
  (footprint "Resistor_SMD:R_0603_1608Metric"
    (layer "F.Cu") (at 20 20)
    (property "Reference" "R2" (at 0 -1) (layer "F.SilkS"))
    (pad "1" smd roundrect (at 0.825 0) (size 0.8 0.95)
      (layers "F.Cu" "F.Paste" "F.Mask") (net 1 "SIG")))
  (segment (start 9.175 20) (end 20.825 20) (width 0.25) (layer "F.Cu") (net 1))
  (gr_line (start 0 0) (end 30 0) (layer "Edge.Cuts") (width 0.1))
  (gr_line (start 0 30) (end 30 30) (layer "Edge.Cuts") (width 0.1))
)

