/**
 * The module's display name.
 *
 * Deliberately not a translation key: it is a product name, and the same
 * string is the `name` in template/module.prop, the `title` in
 * public/config.json, and the document title in index.html. Those files are
 * consumed by the host and by the packaging script, so this constant is the
 * copy the WebUI itself renders.
 */
export const MODULE_NAME = "Oh My Keymint";
