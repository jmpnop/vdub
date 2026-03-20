# vdub — Logo Options (Disney/Pixar Coco Style)

## Source: Coco-style sugar skull (colorful floral)

The Pixar Coco skull — flowers, butterflies, guitars, candles forming a skull shape. Vibrant colors on dark background.

### Original (white bg)

<img src="file:///Users/pasha/RustRoverProjects/vdub/docs/coco_clean.png" width="500">

### Terminal render (dark bg, truecolor half-blocks)

<img src="file:///Users/pasha/RustRoverProjects/vdub/docs/coco_dark.png" width="500">

Available sizes: 50 cols (23 rows), 40 cols (18 rows), 35 cols (15 rows)

---

## Source: Doodle sugar skull set

Hand-drawn style, simpler shapes. Could pick one and colorize it.

<img src="file:///Users/pasha/RustRoverProjects/vdub/docs/doodle_skulls.png" width="600">

---

## Generated: PIL pixel art skull

Programmatic sugar skull with flower crown, petal-ringed eyes, heart nose, teeth.

<img src="file:///Users/pasha/RustRoverProjects/vdub/docs/catrina_large.png" width="500">

---

## Preview in terminal

```bash
# Coco skull (best — colorful, Pixar style)
chafa --format=symbols --size=40 --symbols=half /tmp/coco_dark.png

# Generated pixel art skull
python3 scripts/gen_catrina_v3.py --combo
```
