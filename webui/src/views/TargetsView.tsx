import { useCallback, useEffect, useState } from "react";
import IconAdd from "~icons/material-symbols/add-circle";
import IconClear from "~icons/material-symbols/checklist";
import IconSelectAll from "~icons/material-symbols/done-all";
import IconRefresh from "~icons/material-symbols/refresh";
import { Button } from "../components/atoms/Button";
import { Card } from "../components/atoms/Card";
import { Checkbox } from "../components/atoms/Checkbox";
import { Divider } from "../components/atoms/Divider";
import { Icon } from "../components/atoms/Icon";
import { IconButton } from "../components/atoms/IconButton";
import { ProgressIndicator } from "../components/atoms/ProgressIndicator";
import { SearchBar } from "../components/atoms/SearchBar";
import { BottomSheet } from "../components/molecules/BottomSheet";
import { TopAppBar } from "../components/molecules/TopAppBar";
import { MODULE_NAME } from "../module_info";
import { appList } from "../state/app";
import { useOverlayHistory } from "../state/useOverlayHistory";
import { tr } from "../utils/tr";

const FILTERS = ["all", "selected", "unselected"] as const;
type Filter = (typeof FILTERS)[number];

const FILTER_LABELS: Record<Filter, [string, string]> = {
	all: ["filter_all", "All"],
	selected: ["filter_selected", "Selected"],
	unselected: ["filter_unselected", "Not selected"],
};

export interface TargetsViewProps {
	onClose: () => void;
	onSaved: () => void;
	onNotify: (message: string, error?: boolean) => void;
	onOverlayChange: (open: boolean) => void;
}

/**
 * Scoped package management: the list of apps that route through this
 * keybox. Selection is held by AppList and only committed by Save, so
 * leaving without saving discards the edits.
 */
export function TargetsView({
	onClose,
	onSaved,
	onNotify,
	onOverlayChange,
}: TargetsViewProps) {
	const [snapshot, setSnapshot] = useState(() => appList.getSnapshot());
	const [query, setQuery] = useState("");
	const [filter, setFilter] = useState<Filter>("all");
	const [busy, setBusy] = useState(false);
	const [systemOpen, setSystemOpen] = useState(false);

	useEffect(() => appList.subscribe(setSnapshot), []);

	useEffect(() => {
		onOverlayChange(systemOpen);
	}, [systemOpen, onOverlayChange]);

	useOverlayHistory([
		{
			key: "targets-system",
			open: systemOpen,
			close: () => setSystemOpen(false),
		},
	]);

	// Derived on every render rather than memoised: the source is an in-memory
	// array, and caching it would need the subscription revision as a phantom
	// dependency — which is where staleness bugs come from.
	const entries = appList.getTargetEntries(query, filter);
	const systemEntries = appList.getSystemEntries("");

	const save = useCallback(async () => {
		setBusy(true);
		try {
			await appList.save();
			setBusy(false);
			onNotify(tr("prompt_saved_target", "Config saved"));
			onSaved();
			onClose();
		} catch (error) {
			setBusy(false);
			onNotify(error instanceof Error ? error.message : String(error), true);
		}
	}, [onNotify, onSaved, onClose]);

	return (
		<section
			data-testid="omk-targets"
			className="fixed inset-0 z-30 flex flex-col bg-omk-bg"
			aria-label={tr("app_targets_title", "Add package names")}
		>
			<TopAppBar
				title={MODULE_NAME}
				backLabel={tr("functional_button_back", "Back")}
				onBack={onClose}
				actions={
					<>
						<IconButton
							aria-label={tr("menu_select_all", "Select all")}
							onClick={() => appList.selectAll()}
						>
							<Icon as={IconSelectAll} size="md" />
						</IconButton>
						<IconButton
							aria-label={tr("menu_deselect_all", "Deselect all")}
							onClick={() => appList.deselectAll()}
						>
							<Icon as={IconClear} size="md" />
						</IconButton>
						<IconButton
							aria-label={tr("menu_refresh", "Refresh")}
							disabled={busy}
							onClick={() => void appList.fetch()}
						>
							<Icon as={IconRefresh} size="md" />
						</IconButton>
					</>
				}
			/>

			<div className="flex flex-col gap-2 px-4 py-3">
				<SearchBar
					value={query}
					onChange={setQuery}
					placeholder={tr("search_bar_search_placeholder", "Search")}
					clearLabel={tr("functional_button_cancel", "Cancel")}
				/>
				<div className="flex gap-2">
					{FILTERS.map((value) => {
						const [key, fallback] = FILTER_LABELS[value];
						return (
							<Button
								key={value}
								variant={filter === value ? "primary" : "secondary"}
								aria-pressed={filter === value}
								aria-label={tr(key, fallback)}
								onClick={() => setFilter(value)}
								className="h-8 px-3"
							>
								{tr(key, fallback)}
							</Button>
						);
					})}
				</div>
			</div>

			{/*
			 * A plain container, not <main>: this view is a full-screen overlay
			 * rendered beside the shell's own main landmark, and a document may
			 * only expose one.
			 */}
			<div
				className="min-h-0 flex-1 overflow-y-auto px-4 pb-4"
				aria-busy={busy}
			>
				{entries.length === 0 ? (
					<Card className="flex flex-col items-center gap-2 py-10">
						<Icon as={IconAdd} className="text-omk-muted" />
						<p className="text-omk-caption text-omk-muted">
							{tr("app_targets_empty", "No matching apps")}
						</p>
					</Card>
				) : (
					<Card className="p-0">
						{entries.map((entry, index) => (
							<div key={entry.packageName}>
								{index > 0 ? <Divider inset /> : null}
								<Checkbox
									id={`target-${entry.packageName}`}
									checked={entry.selected}
									onChange={(next) =>
										appList.setSelected(entry.packageName, next)
									}
									className="min-h-14 py-2"
								>
									<span className="flex min-w-0 flex-1 flex-col">
										<span className="truncate text-omk-body-strong text-omk-on">
											{entry.appName}
										</span>
										<span className="truncate text-omk-mono text-omk-muted">
											{entry.packageName}
										</span>
									</span>
								</Checkbox>
							</div>
						))}
					</Card>
				)}

				<Button
					variant="secondary"
					className="mt-4 w-full"
					onClick={() => setSystemOpen(true)}
				>
					<Icon as={IconAdd} size="md" />
					{tr("menu_add_system_app", "Add a system app")}
				</Button>
			</div>

			<div
				style={{ paddingBottom: "calc(var(--omk-bottom-inset) + 12px)" }}
				className="border-omk-divider border-t bg-omk-surface px-4 pt-3"
			>
				<Button
					className="w-full"
					busy={busy}
					disabled={!snapshot.isWritable}
					onClick={() => void save()}
				>
					{tr("functional_button_save", "Save")} ({snapshot.selectedCount})
				</Button>
			</div>

			<BottomSheet
				open={systemOpen}
				onClose={() => setSystemOpen(false)}
				title={tr("add_system_app_title", "System apps")}
			>
				<div className="min-h-0 flex-1 overflow-y-auto pb-2">
					<Card className="p-0">
						{systemEntries.map((entry, index) => (
							<div key={entry.packageName}>
								{index > 0 ? <Divider inset /> : null}
								<Checkbox
									id={`system-${entry.packageName}`}
									checked={entry.selected}
									onChange={() => appList.toggleSelected(entry.packageName)}
									className="min-h-12"
								>
									<span className="min-w-0 flex-1 truncate text-omk-mono text-omk-on">
										{entry.packageName}
									</span>
								</Checkbox>
							</div>
						))}
					</Card>
				</div>
				<Button className="w-full" onClick={() => setSystemOpen(false)}>
					{tr("functional_button_close", "Close")}
				</Button>
			</BottomSheet>

			{busy ? (
				<div className="absolute inset-0 flex items-center justify-center bg-omk-scrim">
					<ProgressIndicator
						aria-label={tr("home_status_loading", "Checking")}
					/>
				</div>
			) : null}
		</section>
	);
}
