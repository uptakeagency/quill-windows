import { useState, useCallback } from 'react';
import type { TechExplanation, ExplanationLevel } from '../lib/types';

interface DrillDownState {
  stack: TechExplanation[];
  cache: Map<string, TechExplanation>;
}

export function useDrillDown() {
  const [state, setState] = useState<DrillDownState>({
    stack: [],
    cache: new Map(),
  });

  const current = state.stack.length > 0 ? state.stack[state.stack.length - 1] : null;
  const breadcrumbs = state.stack.map((e) => e.term);
  const canGoBack = state.stack.length > 1;

  const push = useCallback((explanation: TechExplanation) => {
    setState((prev) => {
      const key = `${explanation.term}:${explanation.level}`;
      const newCache = new Map(prev.cache);
      newCache.set(key, explanation);
      return {
        stack: [...prev.stack, explanation],
        cache: newCache,
      };
    });
  }, []);

  const pop = useCallback(() => {
    setState((prev) => {
      if (prev.stack.length <= 1) return prev;
      return {
        ...prev,
        stack: prev.stack.slice(0, -1),
      };
    });
  }, []);

  const popTo = useCallback((index: number) => {
    setState((prev) => {
      if (index < 0 || index >= prev.stack.length) return prev;
      return {
        ...prev,
        stack: prev.stack.slice(0, index + 1),
      };
    });
  }, []);

  const replaceTop = useCallback((explanation: TechExplanation) => {
    setState((prev) => {
      const key = `${explanation.term}:${explanation.level}`;
      const newCache = new Map(prev.cache);
      newCache.set(key, explanation);
      if (prev.stack.length === 0) {
        return { stack: [explanation], cache: newCache };
      }
      const newStack = [...prev.stack];
      newStack[newStack.length - 1] = explanation;
      return { stack: newStack, cache: newCache };
    });
  }, []);

  const getCached = useCallback(
    (term: string, level: ExplanationLevel): TechExplanation | undefined => {
      return state.cache.get(`${term}:${level}`);
    },
    [state.cache],
  );

  const reset = useCallback(() => {
    setState((prev) => ({ ...prev, stack: [] }));
  }, []);

  const clearCache = useCallback(() => {
    setState({ stack: [], cache: new Map() });
  }, []);

  return {
    current,
    breadcrumbs,
    canGoBack,
    cacheSize: state.cache.size,
    push,
    pop,
    popTo,
    replaceTop,
    getCached,
    reset,
    clearCache,
  };
}
