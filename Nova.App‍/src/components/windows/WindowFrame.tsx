import type { ReactNode } from "react";
import { FloatingPanel } from "@/components/nova/floating-panel";
import type { WindowMetadata } from "@/types/window";

type WindowFrameProps = {
    metadata: WindowMetadata;
    children: ReactNode;
};

export function WindowFrame({ metadata, children }: WindowFrameProps) {
    const Icon = metadata.Icon;

    return (
        <FloatingPanel
            id={metadata.id}
            name={metadata.title}
            icon={<Icon aria-hidden="true" color={metadata.theme.accent} />}
            initialPosition={metadata.initialPosition}
            initialSize={metadata.initialSize}
            initiallyOpen={metadata.initiallyOpen}>
            {children}
        </FloatingPanel>
    );
}
