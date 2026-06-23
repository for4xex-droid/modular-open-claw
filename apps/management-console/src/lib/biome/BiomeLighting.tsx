interface BiomeLightingProps {
  rarity?: number;
}

export function BiomeLighting({ rarity = 0 }: BiomeLightingProps) {
  // レアリティが高くなると、少し光を強く/色味を変える
  const ambientIntensity = 0.4 + rarity * 0.05;
  const directionalIntensity = 0.8 + rarity * 0.1;
  const lightColor = rarity >= 4 ? '#fff0d0' : '#ffffff'; // Legendaryは少し温かみのある光

  return (
    <group>
      <ambientLight intensity={ambientIntensity} />
      <directionalLight 
        position={[40, 80, 100]} 
        intensity={directionalIntensity} 
        color={lightColor}
      />
    </group>
  );
}
