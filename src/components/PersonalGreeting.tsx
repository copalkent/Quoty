import React from "react";

interface PersonalGreetingProps {
  displayName?: string | null;
}

export const PersonalGreeting = React.memo(function PersonalGreeting({
  displayName,
}: PersonalGreetingProps) {
  const trimmed = displayName?.trim();

  return (
    <div className="personal-greeting" aria-label="Greeting">
      {trimmed ? (
        <span className="greeting-text">
          Hi, <strong className="greeting-name">{trimmed}</strong>
        </span>
      ) : (
        <span className="greeting-text greeting-neutral">Hi there</span>
      )}
    </div>
  );
});
