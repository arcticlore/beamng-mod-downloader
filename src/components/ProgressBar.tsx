export function ProgressBar({
  received,
  total,
}: {
  received: number;
  total: number | null;
}) {
  const pct = total && total > 0 ? Math.min(100, (received / total) * 100) : 0;
  return (
    <div className="progress">
      <div className="progress-fill" style={{ width: `${(pct || 0).toFixed(1)}%` }} />
    </div>
  );
}