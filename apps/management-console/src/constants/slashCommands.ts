/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

export interface SlashCommandDef {
    readonly cmd: string;
    readonly label: string;
    readonly desc: string;
    readonly iconName: 'Volume2' | 'Sparkles' | 'Brain' | 'Cpu';
    readonly envelopeType: string | null;  // null = /clear (特殊コマンド)
}

export const SLASH_COMMANDS: readonly SlashCommandDef[] = [
    { cmd: '/store',    label: 'Voice Store',   desc: 'Open Voice & Asset Store',   iconName: 'Volume2',  envelopeType: 'voiceStore' },
    { cmd: '/treasure', label: 'Treasure Box',  desc: 'Open Gacha & Rewards',       iconName: 'Sparkles', envelopeType: 'treasureItem' },
    { cmd: '/lora',     label: 'LoRA Market',   desc: 'Explore Fine-tuned Models',  iconName: 'Brain',    envelopeType: 'loraMarket' },
    { cmd: '/clear',    label: 'Clear Chat',    desc: 'Clear history',              iconName: 'Cpu',      envelopeType: null },
] as const;
