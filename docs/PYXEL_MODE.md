<!-- SPDX-License-Identifier: MPL-2.0 -->
# Pyxel sprite workspace (phase 1)

Choose **File → New Pyxel 32×32**. The Color panel offers the 16 colors
from Pyxel's default palette and a transparent eraser. Drag in the pixel
canvas to edit the visible frame. Use **+ Blank frame** or **+ Duplicate
frame**, then select a numbered frame to continue editing. **Delete frame**
removes the current frame (Undo restores it).

Save the project as `.efude`. Each frame is stored as a raster layer, so
the frames survive closing and reopening the document. **File → Export →
PNG** exports only the currently visible 32×32 frame with transparency.
Export each frame separately if you need several PNGs. Sprite-sheet export,
playback and onion skin are future work.

The dedicated pixel canvas always writes an exact default Pyxel color or
transparent pixel. Illustration controls are hidden for Pyxel documents.
Before saving or exporting, the app checks every frame for custom colors,
partial alpha, incompatible layer properties and canvas size. If another
operation changes the sprite outside these limits, the app shows an error
and does not save/export until that change is undone.

The default palette can be replaced by a game at runtime. This workspace
intentionally targets the [official default palette](https://github.com/kitao/pyxel/blob/main/docs/pyxel.gpl).
