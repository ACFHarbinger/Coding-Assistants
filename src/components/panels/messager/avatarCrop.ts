/** Fixed output of the crop tool. 256² is enough for 24–28px avatars at
 * 4× and for larger dashboard tiles, without storing a full-res photo. */
export const CROP_OUTPUT_SIZE = 256;

export const MIN_CROP_PX = 32;

export type CropRect = { x: number; y: number; size: number };

export type CropCorner = "nw" | "ne" | "sw" | "se";

export function initialCrop(imageWidth: number, imageHeight: number): CropRect {
  const size = Math.max(1, Math.min(imageWidth, imageHeight));
  return {
    x: (imageWidth - size) / 2,
    y: (imageHeight - size) / 2,
    size,
  };
}

export function clampCrop(crop: CropRect, imageWidth: number, imageHeight: number): CropRect {
  const maxSize = Math.max(1, Math.min(imageWidth, imageHeight));
  const minSize = Math.min(MIN_CROP_PX, maxSize);
  const size = Math.max(minSize, Math.min(crop.size, maxSize));
  return {
    x: Math.max(0, Math.min(crop.x, Math.max(0, imageWidth - size))),
    y: Math.max(0, Math.min(crop.y, Math.max(0, imageHeight - size))),
    size,
  };
}

export function moveCrop(
  crop: CropRect,
  dx: number,
  dy: number,
  imageWidth: number,
  imageHeight: number,
): CropRect {
  return clampCrop({ ...crop, x: crop.x + dx, y: crop.y + dy }, imageWidth, imageHeight);
}

/** Resize while keeping 1:1. The opposite corner is the anchor. Dragging
 * past an image edge shrinks the square so it stays inside; crossing the
 * anchor does not flip the box (size floors at `MIN_CROP_PX`). */
export function resizeCropFromCorner(
  crop: CropRect,
  corner: CropCorner,
  pointerX: number,
  pointerY: number,
  imageWidth: number,
  imageHeight: number,
): CropRect {
  const maxSize = Math.max(1, Math.min(imageWidth, imageHeight));
  const minSize = Math.min(MIN_CROP_PX, maxSize);
  const east = crop.x + crop.size;
  const south = crop.y + crop.size;

  let size: number;
  let x: number;
  let y: number;
  switch (corner) {
    case "se": {
      const ax = crop.x;
      const ay = crop.y;
      size = Math.min(pointerX - ax, pointerY - ay, imageWidth - ax, imageHeight - ay);
      x = ax;
      y = ay;
      break;
    }
    case "nw": {
      const ax = east;
      const ay = south;
      size = Math.min(ax - pointerX, ay - pointerY, ax, ay);
      x = ax - Math.max(minSize, size);
      y = ay - Math.max(minSize, size);
      break;
    }
    case "ne": {
      const ax = crop.x;
      const ay = south;
      size = Math.min(pointerX - ax, ay - pointerY, imageWidth - ax, ay);
      x = ax;
      y = ay - Math.max(minSize, size);
      break;
    }
    case "sw": {
      const ax = east;
      const ay = crop.y;
      size = Math.min(ax - pointerX, pointerY - ay, ax, imageHeight - ay);
      x = ax - Math.max(minSize, size);
      y = ay;
      break;
    }
  }

  return clampCrop({ x, y, size: Math.max(minSize, size) }, imageWidth, imageHeight);
}

export function encodeCroppedPng(image: HTMLImageElement, crop: CropRect): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = CROP_OUTPUT_SIZE;
  canvas.height = CROP_OUTPUT_SIZE;
  const ctx = canvas.getContext("2d");
  if (!ctx) return Promise.reject(new Error("canvas 2d context unavailable"));
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.drawImage(
    image,
    crop.x,
    crop.y,
    crop.size,
    crop.size,
    0,
    0,
    CROP_OUTPUT_SIZE,
    CROP_OUTPUT_SIZE,
  );
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("failed to encode cropped avatar"));
        return;
      }
      const reader = new FileReader();
      reader.onerror = () => reject(reader.error ?? new Error("failed to read cropped avatar"));
      reader.onload = () => {
        const result = String(reader.result || "");
        const comma = result.indexOf(",");
        resolve(comma >= 0 ? result.slice(comma + 1) : result);
      };
      reader.readAsDataURL(blob);
    }, "image/png");
  });
}
