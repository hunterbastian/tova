export function createInteractableSystem() {
  const interactables = new Map();

  function clear() {
    interactables.clear();
  }

  function register(interactable) {
    interactables.set(interactable.id, interactable);
    return interactable.id;
  }

  function getFocused(playerPosition) {
    let nearest = null;

    for (const interactable of interactables.values()) {
      if (interactable.isAvailable && !interactable.isAvailable()) {
        continue;
      }

      const radius = interactable.radius ?? 3;
      const distance = playerPosition.distanceTo(interactable.position);
      if (distance > radius) {
        continue;
      }

      if (!nearest || distance < nearest.distance) {
        nearest = { interactable, distance };
      }
    }

    return nearest;
  }

  function interact(playerPosition) {
    const focused = getFocused(playerPosition);
    if (!focused) {
      return false;
    }

    return Boolean(focused.interactable.onInteract?.(focused));
  }

  function getPrompt(playerPosition) {
    const focused = getFocused(playerPosition);
    return focused?.interactable.prompt ?? "";
  }

  function getDebugState(playerPosition) {
    const focused = getFocused(playerPosition);

    return {
      count: interactables.size,
      active: focused
        ? {
            id: focused.interactable.id,
            label: focused.interactable.label,
            distance: Number(focused.distance.toFixed(2)),
          }
        : null,
    };
  }

  return {
    clear,
    getDebugState,
    getPrompt,
    interact,
    register,
  };
}
