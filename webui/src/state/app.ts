import { AppList } from "../app_list/app_list";
import { Cli } from "../cli";
import { ConfigOhMyKeyMint } from "../config_ohmykeymint";
import { FileSelector } from "../file_selector/file_selector";
import { History } from "../history";

/*
 * Single instances for the whole WebUI. These are plain classes rather than
 * React context because they own lifetimes that outlive any component: the
 * back stack, the package list cache, and the file picker session.
 */
export const cli = new Cli();
export const config = new ConfigOhMyKeyMint(cli);
export const appList = new AppList(config);
export const fileSelector = new FileSelector();
export const history = new History();
