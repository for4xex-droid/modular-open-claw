/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 *
 * Phase E（VRM 配線計画）で使用予定 — 現時点では import ゼロ。実行時アバターは PNG ビルボード。
 */
import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { VRM } from '@pixiv/three-vrm';

import { useVisemeSync } from '../../hooks/useVisemeSync';

/** VRM 口形素（母音）表現名。speaking ケース内で共有。 */
const VOWELS = ['aa', 'ih', 'ou', 'ee', 'oh'] as const;

export const useVrmExpression = (vrm: VRM | null, avatarState: string) => {
    const blinkTimerRef = useRef(3 + Math.random() * 3);
    const isBlinkingRef = useRef(false);
    const blinkDurationRef = useRef(0);
    const lipIndexRef = useRef(0);
    const lipTimerRef = useRef(0);

    const visemeSync = useVisemeSync();

    useFrame((_, delta) => {
        if (!vrm) return;

        const elapsed = vrm.scene.userData.elapsed ?? 0;
        vrm.scene.userData.elapsed = elapsed + delta;

        const em = vrm.expressionManager;
        if (!em) return;

        // 1. Helper to reset
        const resetExpressions = () => {
            em.setValue('happy', 0);
            em.setValue('angry', 0);
            em.setValue('sad', 0);
            em.setValue('relaxed', 0);
            em.setValue('surprised', 0);
            em.setValue('aa', 0);
            em.setValue('ih', 0);
            em.setValue('ou', 0);
            em.setValue('ee', 0);
            em.setValue('oh', 0);
        };

        // 2. Auto Blink (with double blink chance)
        if (isBlinkingRef.current) {
            blinkDurationRef.current += delta;
            if (blinkDurationRef.current >= 0.12) {
                em.setValue('blink', 0);
                isBlinkingRef.current = false;
                blinkDurationRef.current = 0;
                // Double blink chance
                if (Math.random() > 0.85) {
                    blinkTimerRef.current = 0.05;
                } else {
                    blinkTimerRef.current = 2 + Math.random() * 5;
                }
            } else {
                em.setValue('blink', 1);
            }
        } else {
            blinkTimerRef.current -= delta;
            if (blinkTimerRef.current <= 0) {
                isBlinkingRef.current = true;
            }
        }

        // 3. State-based Logic
        switch (avatarState) {
            case 'thinking':
                resetExpressions();
                em.setValue('surprised', 0.4);
                if (vrm.humanoid) {
                    const head = vrm.humanoid.getNormalizedBoneNode('head');
                    const neck = vrm.humanoid.getNormalizedBoneNode('neck');
                    if (head) head.rotation.z = Math.sin(elapsed * 0.4) * 0.06;
                    if (neck) neck.rotation.z = Math.sin(elapsed * 0.4) * 0.04;
                }
                break;

            case 'speaking': {
                resetExpressions();
                em.setValue('happy', 0.2);

                const { viseme, weight, isActive } = visemeSync.tick(delta);

                if (isActive) {
                    // Procedural lip-sync based on queued visemes
                    VOWELS.forEach(v => em.setValue(v, 0));

                    if (viseme && (VOWELS as readonly string[]).includes(viseme)) {
                        em.setValue(viseme, weight);
                        // Dispatch for UI visualization
                        window.dispatchEvent(new CustomEvent('aiome_viseme_played', { detail: { viseme: viseme.toUpperCase() } }));
                    }
                } else {
                    // Fallback to random lip-sync if no visemes provided
                    lipTimerRef.current -= delta;
                    if (lipTimerRef.current <= 0) {
                        VOWELS.forEach(v => em.setValue(v, 0));
                        const vowel = VOWELS[lipIndexRef.current % VOWELS.length];
                        em.setValue(vowel, 0.6 + Math.random() * 0.4);
                        lipIndexRef.current++;
                        lipTimerRef.current = 0.08 + Math.random() * 0.12;
                    }
                }
                break;
            }

            case 'learning':
                resetExpressions();
                em.setValue('relaxed', 0.6);
                if (vrm.humanoid) {
                    const head = vrm.humanoid.getNormalizedBoneNode('head');
                    if (head) head.rotation.x = 0.1 + Math.sin(elapsed * 2) * 0.05;
                }
                break;

            case 'meditating':
                resetExpressions();
                em.setValue('blink', 0.9);
                em.setValue('relaxed', 0.4);
                break;

            case 'awakened':
                resetExpressions();
                em.setValue('happy', 0.9);
                em.setValue('surprised', 0.2);
                break;

            default: // idle
                resetExpressions();
                if (vrm.humanoid) {
                    const head = vrm.humanoid.getNormalizedBoneNode('head');
                    const neck = vrm.humanoid.getNormalizedBoneNode('neck');
                    if (head) {
                        head.rotation.y = Math.sin(elapsed * 0.4) * 0.1;
                        head.rotation.x = Math.sin(elapsed * 0.3) * 0.04;
                    }
                    if (neck) {
                        neck.rotation.y = Math.sin(elapsed * 0.4) * 0.05;
                    }
                }
                break;
        }
    });
};
