import { DownloadIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const installGameWindowMetadata: WindowMetadata = {
    id: "install-game",
    title: "安装新游戏",
    Icon: DownloadIcon,
    theme: {
        accent: "#a5b4fc",
        tile: "linear-gradient(180deg, #4338ca 0%, #312e81 100%)",
        ink: "#e0e7ff",
    },
    initialPosition: { x: 24, y: 52 },
};

export function InstallGameWindow() {
    return (
        <WindowFrame metadata={installGameWindowMetadata}>
            <WindowEmptyState icon={<DownloadIcon />} label="尚未选择游戏安装包" />
        </WindowFrame>
    );
}
