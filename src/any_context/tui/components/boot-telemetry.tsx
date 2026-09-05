import React, { useState, useEffect, useRef } from "react";
import type { BootTelemetryStep } from "../bridge-client";
import { anyContextTheme } from "../themes";

const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const MIN_STEP_CADENCE_MS = 70; // Cadência perceptual mínima da Abordagem B

interface BootTelemetryProps {
  steps: BootTelemetryStep[];
}

function formatElapsed(ms?: number): string {
  if (ms === undefined || ms === null) return "  done ";
  if (ms >= 1000) {
    const s = (ms / 1000).toFixed(2) + "s";
    return s.padStart(6, " ");
  }
  const m = ms.toFixed(1) + "ms";
  return m.padStart(6, " ");
}

export const BootTelemetry = ({ steps: incomingSteps }: BootTelemetryProps): any => {
  const [spinnerIndex, setSpinnerIndex] = useState(0);
  const [displaySteps, setDisplaySteps] = useState<BootTelemetryStep[]>(incomingSteps);
  const stepStartTimesRef = useRef<Record<string, number>>({});
  const incomingStepsRef = useRef<BootTelemetryStep[]>(incomingSteps);
  incomingStepsRef.current = incomingSteps;

  // Initialize step start times
  useEffect(() => {
    incomingSteps.forEach((s) => {
      if (s.status === "running" && !stepStartTimesRef.current[s.id]) {
        stepStartTimesRef.current[s.id] = Date.now();
      }
    });
  }, [incomingSteps]);

  // Smooth cascading cadence engine (Abordagem B)
  useEffect(() => {
    const timer = setInterval(() => {
      setDisplaySteps((prevVisual) => {
        const latestIncoming = incomingStepsRef.current;
        const nextVisual = prevVisual.map((p) => ({ ...p }));
        const now = Date.now();

        let canAdvance = true;

        for (let i = 0; i < latestIncoming.length; i++) {
          const target = latestIncoming[i];
          const current = nextVisual[i] || { ...target, status: "pending" };

          if (target.status === "done") {
            const startedAt = stepStartTimesRef.current[target.id] || now;
            const elapsedSinceStart = now - startedAt;

            // If this step has been visible for at least MIN_STEP_CADENCE_MS (or if it's runtime which already took seconds)
            if (current.status === "done" || elapsedSinceStart >= MIN_STEP_CADENCE_MS || target.id === "runtime") {
              nextVisual[i] = { ...target };
            } else {
              // Keep running with spinner for minimum cadence
              if (current.status !== "running") {
                nextVisual[i] = { ...current, status: "running" };
                stepStartTimesRef.current[target.id] = now;
              }
              canAdvance = false;
              break;
            }
          } else if (target.status === "running") {
            if (current.status !== "running") {
              nextVisual[i] = { ...target, status: "running" };
              stepStartTimesRef.current[target.id] = now;
            } else {
              nextVisual[i] = { ...target, status: "running" };
            }
            canAdvance = false;
            break;
          } else {
            // Pending step
            if (canAdvance && i > 0 && nextVisual[i - 1]?.status === "done") {
              // Anticipate next step into running
              nextVisual[i] = { ...target, status: "running" };
              stepStartTimesRef.current[target.id] = now;
              canAdvance = false;
              break;
            } else {
              nextVisual[i] = { ...target };
            }
          }
        }

        return nextVisual;
      });
    }, 35);

    return () => clearInterval(timer);
  }, []);

  // Spinner animation for running steps
  const hasRunningStep = displaySteps.some((s) => s.status === "running");
  useEffect(() => {
    if (!hasRunningStep) return;
    const interval = setInterval(() => {
      setSpinnerIndex((prev) => (prev + 1) % SPINNER_FRAMES.length);
    }, 80);
    return () => clearInterval(interval);
  }, [hasRunningStep]);

  const spinnerChar = SPINNER_FRAMES[spinnerIndex];

  // Only render if we have steps to display
  if (!displaySteps || displaySteps.length === 0) {
    return null;
  }

  return (
    <box flexDirection="column" marginTop={1} marginBottom={1} flexShrink={0}>
      {/* Telemetry Tree Header */}
      <text fg={anyContextTheme.foregroundMuted}>
        {"  ┌─ "}
        <span style={{ fg: anyContextTheme.accent }}>
          <b>{"⚡ Engine Startup Telemetry"}</b>
        </span>
      </text>

      {/* Telemetry Tree Steps */}
      {displaySteps.map((step, idx) => {
        const isLast = idx === displaySteps.length - 1;
        const branchSymbol = isLast ? "  │ └─ " : "  │ ├─ ";

        let timeTagText = `[ ${spinnerChar} ] `;
        let timeTagColor = anyContextTheme.accentWarning;
        let labelColor = anyContextTheme.foreground;

        if (step.status === "done") {
          const timeFormatted = formatElapsed(step.elapsed_ms);
          timeTagText = `[${timeFormatted}] `;
          timeTagColor =
            step.elapsed_ms !== undefined && step.elapsed_ms < 100
              ? anyContextTheme.accentSuccess
              : anyContextTheme.accentWarning;

          if (isLast || step.id === "ready") {
            labelColor = anyContextTheme.accentSuccess;
          }
        } else if (step.status === "pending") {
          timeTagText = "[   ·   ] ";
          timeTagColor = anyContextTheme.foregroundMuted;
          labelColor = anyContextTheme.foregroundMuted;
        } else if (step.status === "error") {
          timeTagText = "[  ERR  ] ";
          timeTagColor = anyContextTheme.accentError;
          labelColor = anyContextTheme.accentError;
        }

        return (
          <text key={step.id}>
            <span style={{ fg: anyContextTheme.foregroundMuted }}>{branchSymbol}</span>
            <span style={{ fg: timeTagColor }}>
              <b>{timeTagText}</b>
            </span>
            <span style={{ fg: labelColor }}>
              {isLast && step.status === "done" ? <b>{step.label}</b> : step.label}
            </span>
          </text>
        );
      })}
    </box>
  );
};
