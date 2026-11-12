import { GraphStarIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const multiplayerWindowMetadata: WindowMetadata = {
    id: "multiplayer",
    title: "联机",
    Icon: GraphStarIcon,
    theme: {
        accent: "#fda4af",
        tile: "linear-gradient(180deg, #e11d48 0%, #4c0519 100%)",
        ink: "#ffe4e6",
    },
    initialPosition: { x: 24, y: 108 },
};

export function MultiplayerWindow() {
    return (
        <WindowFrame metadata={multiplayerWindowMetadata}>
            <WindowEmptyState icon={<GraphStarIcon />} label="暂无联机记录" />
        </WindowFrame>
    );
}
