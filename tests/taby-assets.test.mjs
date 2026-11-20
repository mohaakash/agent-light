import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "assets", "taby");
const manifest = JSON.parse(readFileSync(join(root, "catalog.json"), "utf8"));

const requiredFaceAnimations = [
  "idle_01_loop",
  "idle_02_loop",
  "calm_idle_01_f000",
  "working_laptop_loop",
  "working_laptop_excited_loop",
  "calm_work_medium_f000",
  "waiting_01",
  "angry_01_loop",
  "angry_02_loop",
  "task_completed",
  "confirmation",
  "thumbs_up",
];

test("Taby animation catalog contains the required status-face clips", () => {
  const ids = new Set(manifest.animations.map((animation) => animation.id));
  for (const id of requiredFaceAnimations) assert.ok(ids.has(id), `missing ${id}`);
});

test("every bundled Taby GIF matches its catalog hash, size, and dimensions", () => {
  assert.equal(manifest.display.width, 280);
  assert.equal(manifest.display.height, 456);
  for (const animation of manifest.animations) {
    const file = join(root, animation.relative_path);
    const bytes = readFileSync(file);
    assert.equal(bytes.length, animation.byte_length, `${animation.id}: byte length`);
    assert.equal(createHash("sha256").update(bytes).digest("hex"), animation.sha256, `${animation.id}: SHA-256`);
    assert.equal(bytes.subarray(0, 3).toString("ascii"), "GIF", `${animation.id}: GIF signature`);
    assert.equal(bytes.readUInt16LE(6), manifest.display.width, `${animation.id}: width`);
    assert.equal(bytes.readUInt16LE(8), manifest.display.height, `${animation.id}: height`);
    assert.ok(animation.duration_ms > 0, `${animation.id}: duration`);
    assert.ok(["loop", "play_once"].includes(animation.loop_policy), `${animation.id}: loop policy`);

    const poster = readFileSync(join(root, animation.poster_path));
    assert.equal(poster.length, animation.poster_byte_length, `${animation.id}: poster byte length`);
    assert.equal(poster.subarray(0, 4).toString("ascii"), "RIFF", `${animation.id}: poster container`);
    assert.equal(poster.subarray(8, 12).toString("ascii"), "WEBP", `${animation.id}: WebP poster`);
  }
});
