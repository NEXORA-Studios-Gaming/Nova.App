import type { ComponentType } from "react";
import type { LucideIcon } from "lucide-react";
import type { DockItemTheme } from "@/types/dock";
import type { WindowPosition, WindowSize } from "@/stores/window-store";

export type WindowMetadata = {
    id: string;
    title: string;
    Icon: LucideIcon;
    theme: DockItemTheme;
    initialPosition: WindowPosition;
    initialSize?: WindowSize;
    initiallyOpen?: boolean;
};

export type WindowModule = {
    metadata: WindowMetadata;
    Component: ComponentType;
};
