import type { WindowModule } from "@/types/window";
import { AppsWindow, appsWindowMetadata } from "@/windows/AppsWindow";
import { DownloadDataPacksWindow, downloadDataPacksWindowMetadata } from "@/windows/DownloadDataPacksWindow";
import { DownloadModsWindow, downloadModsWindowMetadata } from "@/windows/DownloadModsWindow";
import { DownloadModpacksWindow, downloadModpacksWindowMetadata } from "@/windows/DownloadModpacksWindow";
import { DownloadResourcePacksWindow, downloadResourcePacksWindowMetadata } from "@/windows/DownloadResourcePacksWindow";
import { DownloadShadersWindow, downloadShadersWindowMetadata } from "@/windows/DownloadShadersWindow";
import { DownloadWorldsWindow, downloadWorldsWindowMetadata } from "@/windows/DownloadWorldsWindow";
import { HomeWindow, homeWindowMetadata } from "@/windows/HomeWindow";
import { InstallGameWindow, installGameWindowMetadata } from "@/windows/InstallGameWindow";
import { MultiplayerWindow, multiplayerWindowMetadata } from "@/windows/MultiplayerWindow";
import { SettingsWindow, settingsWindowMetadata } from "@/windows/SettingsWindow";
import { TaskManagerWindow, taskManagerWindowMetadata } from "@/windows/TaskManagerWindow";

export const WINDOW_REGISTRY: WindowModule[] = [
    { metadata: homeWindowMetadata, Component: HomeWindow },
    { metadata: appsWindowMetadata, Component: AppsWindow },
    { metadata: installGameWindowMetadata, Component: InstallGameWindow },
    { metadata: downloadModsWindowMetadata, Component: DownloadModsWindow },
    { metadata: downloadModpacksWindowMetadata, Component: DownloadModpacksWindow },
    { metadata: downloadResourcePacksWindowMetadata, Component: DownloadResourcePacksWindow },
    { metadata: downloadShadersWindowMetadata, Component: DownloadShadersWindow },
    { metadata: downloadDataPacksWindowMetadata, Component: DownloadDataPacksWindow },
    { metadata: downloadWorldsWindowMetadata, Component: DownloadWorldsWindow },
    { metadata: multiplayerWindowMetadata, Component: MultiplayerWindow },
    { metadata: settingsWindowMetadata, Component: SettingsWindow },
    { metadata: taskManagerWindowMetadata, Component: TaskManagerWindow },
];

export const WINDOW_METADATA = WINDOW_REGISTRY.map(({ metadata }) => metadata);

export function WindowLayer() {
    return (
        <>
            {WINDOW_REGISTRY.map(({ metadata, Component }) => (
                <Component key={metadata.id} />
            ))}
        </>
    );
}
