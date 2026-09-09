import type { ReactNode } from 'react';

export interface Column<T> {
  key: string;
  header: string;
  numeric?: boolean;
  render: (row: T) => ReactNode;
}

interface DataTableProps<T> {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T, index: number) => string;
  emptyTitle?: string;
  emptyHint?: string;
}

/** Readable table with an explicit empty state. No business logic. */
export default function DataTable<T>({
  columns,
  rows,
  rowKey,
  emptyTitle = 'No rows',
  emptyHint,
}: DataTableProps<T>) {
  if (rows.length === 0) {
    return (
      <div className="card">
        <div className="state">
          <div className="state-title">{emptyTitle}</div>
          {emptyHint && <div className="state-hint">{emptyHint}</div>}
        </div>
      </div>
    );
  }
  return (
    <div className="table-wrap">
      <table className="data">
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={col.key} className={col.numeric ? 'num' : undefined}>
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={rowKey(row, i)}>
              {columns.map((col) => (
                <td key={col.key} className={col.numeric ? 'num' : undefined}>
                  {col.render(row)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
