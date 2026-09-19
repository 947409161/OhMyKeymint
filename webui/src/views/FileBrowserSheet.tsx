import { useEffect, useState } from "react";
import IconParent from "~icons/material-symbols/arrow-upward";
import IconFile from "~icons/material-symbols/description";
import IconFolder from "~icons/material-symbols/folder";
import IconImport from "~icons/material-symbols/input";
import { Button } from "../components/atoms/Button";
import { Card } from "../components/atoms/Card";
import { Divider } from "../components/atoms/Divider";
import { Icon } from "../components/atoms/Icon";
import { IconButton } from "../components/atoms/IconButton";
import { ProgressIndicator } from "../components/atoms/ProgressIndicator";
import { BottomSheet } from "../components/molecules/BottomSheet";
import { fileSelector } from "../state/app";
import { useOverlayHistory } from "../state/useOverlayHistory";
import { tr } from "../utils/tr";

interface BrowserState {
	open: boolean;
	entries: readonly { name: string; isDirectory: boolean }[];
	loading: boolean;
	status: string;
	path: string;
	canGoBack: boolean;
}

function readState(): BrowserState {
	return {
		open: fileSelector.open,
		entries: fileSelector.entries,
		loading: fileSelector.loading,
		status: fileSelector.status,
		path: fileSelector.currentPath,
		canGoBack: fileSelector.canGoBack,
	};
}

/**
 * Shared-storage fallback for the keybox picker. It is only reached when the
 * WebView refuses to open the system document picker; the picker itself is
 * the primary path because it can reach any provider the device has.
 */
export interface FileBrowserSheetProps {
	/** Lets the shell know a modal layer is up so Escape closes one layer. */
	onOpenChange: (open: boolean) => void;
}

export function FileBrowserSheet({ onOpenChange }: FileBrowserSheetProps) {
	const [state, setState] = useState<BrowserState>(readState);

	useEffect(
		() =>
			fileSelector.subscribe(() => {
				setState(readState());
				onOpenChange(fileSelector.open);
			}),
		[onOpenChange],
	);

	useOverlayHistory([
		{
			key: "file-selector",
			open: state.open,
			close: () => fileSelector.cancel(),
		},
	]);

	return (
		<BottomSheet
			open={state.open}
			onClose={() => {
				if (!state.loading) fileSelector.cancel();
			}}
			title={tr("replace_keybox_storage_root", "Shared storage")}
			startAction={
				<IconButton
					aria-label={tr("replace_keybox_storage_parent", "Parent folder")}
					disabled={state.loading || !state.canGoBack}
					onClick={() => fileSelector.navigateBack()}
				>
					<Icon as={IconParent} size="md" />
				</IconButton>
			}
			endAction={
				<IconButton
					aria-label={tr("replace_keybox_open_system", "Open with another app")}
					disabled={state.loading}
					onClick={() => void fileSelector.openWithAnotherApp()}
				>
					<Icon as={IconImport} size="md" />
				</IconButton>
			}
		>
			<p dir="ltr" className="text-omk-mono text-omk-muted wrap-anywhere">
				{state.path}
			</p>

			<div className="flex min-h-0 flex-1 flex-col py-2">
				{state.loading ? (
					<div
						role="status"
						className="flex flex-1 flex-col items-center justify-center gap-2 text-omk-caption text-omk-muted"
					>
						<ProgressIndicator aria-label={state.status || "Loading"} />
						<span>{state.status}</span>
					</div>
				) : null}

				{!state.loading && state.status !== "" ? (
					<p
						role="status"
						className="px-1 py-2 text-omk-caption text-omk-on"
						data-tone="error"
					>
						{state.status}
					</p>
				) : null}

				{!state.loading && state.entries.length === 0 && state.status === "" ? (
					<p
						role="status"
						className="py-6 text-center text-omk-caption text-omk-muted"
					>
						{tr("replace_keybox_storage_empty", "This folder is empty")}
					</p>
				) : null}

				{state.entries.length > 0 ? (
					<Card padded={false} className="omk-scroll min-h-0 flex-1">
						{state.entries.map((entry, index) => (
							<div key={entry.name}>
								{index > 0 ? <Divider inset /> : null}
								<button
									type="button"
									onClick={() => fileSelector.activate(entry)}
									className="flex min-h-12 w-full items-center gap-3 px-4 text-start active:bg-omk-container-high"
								>
									<Icon
										as={entry.isDirectory ? IconFolder : IconFile}
										size="md"
										className="text-omk-muted"
									/>
									<span className="min-w-0 flex-1 truncate text-omk-body text-omk-on">
										{entry.name}
									</span>
								</button>
							</div>
						))}
					</Card>
				) : null}
			</div>

			<Button
				variant="secondary"
				className="w-full"
				disabled={state.loading}
				onClick={() => fileSelector.cancel()}
			>
				{tr("functional_button_cancel", "Cancel")}
			</Button>
		</BottomSheet>
	);
}
