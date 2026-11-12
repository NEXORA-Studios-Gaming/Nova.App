import { LayoutGridIcon } from "lucide-react";
import { WindowEmptyState } from "@/components/windows/WindowEmptyState";
import { WindowFrame } from "@/components/windows/WindowFrame";
import type { WindowMetadata } from "@/types/window";

export const appsWindowMetadata: WindowMetadata = {
    id: "apps",
    title: "Apps",
    Icon: LayoutGridIcon,
    theme: {
        accent: "#a3e635",
        tile: "linear-gradient(180deg, #4d7c0f 0%, #365314 100%)",
        ink: "#ecfccb",
    },
    initialPosition: { x: 24, y: 44 },
};

export function AppsWindow() {
    return (
        <WindowFrame metadata={appsWindowMetadata}>
            <WindowEmptyState icon={<LayoutGridIcon />} label="暂无应用" />
        </WindowFrame>
    );
}
