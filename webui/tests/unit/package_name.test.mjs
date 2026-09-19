import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { isValidPackageName, normalizePackageNames } from "../../src/package_name.ts";

describe("isValidPackageName", () => {
	test("accepts dotted Android package names", () => {
		for (const name of ["a", "com.example", "com.example.app_2", "A.B_C.1"]) {
			assert.equal(isValidPackageName(name), true, name);
		}
	});

	test("rejects malformed names", () => {
		for (const name of ["", ".", "com.", ".com", "com..example", "com/example", "com-example", "com example"]) {
			assert.equal(isValidPackageName(name), false, name);
		}
	});

	test("rejects non-strings and over-long names", () => {
		assert.equal(isValidPackageName(undefined), false);
		assert.equal(isValidPackageName(1), false);
		assert.equal(isValidPackageName("a".repeat(256)), false);
		assert.equal(isValidPackageName("a".repeat(255)), true);
	});
});

describe("normalizePackageNames", () => {
	test("deduplicates while preserving order", () => {
		assert.deepEqual(normalizePackageNames(["b", "a", "b"]), ["b", "a"]);
	});

	test("accepts an empty list", () => {
		assert.deepEqual(normalizePackageNames([]), []);
	});

	test("rejects a non-array payload", () => {
		assert.throws(() => normalizePackageNames("com.example"), /Invalid package list/);
	});

	test("rejects a list containing an invalid entry", () => {
		assert.throws(() => normalizePackageNames(["com.example", "bad name"]), /Invalid package list/);
	});
});
