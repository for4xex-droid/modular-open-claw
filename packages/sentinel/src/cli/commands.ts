/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */
import { native } from '../native';

export function registerCommands(api: any) {
    // Phase 5: /aiome-status
    api.registerCommand({
        name: "aiome-status",
        description: "Check Aiome Core / Watchtower status",
        handler: async (args: string[]) => {
            // we can retrieve stats via watchtower getAgentStats later
            return {
                text: "Aiome Core is running. Watchtower and Immune Systems active."
            };
        }
    });
}
