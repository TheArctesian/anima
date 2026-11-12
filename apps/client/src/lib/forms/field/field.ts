/** What a Field hands its control, to spread onto the input: the id its label points at, and its description and validity. */
export interface ControlLinks {
  id: string;
  'aria-describedby': string | undefined;
  'aria-invalid': true | undefined;
}

/** The control's attributes for a field `id`: the constraint and the error, when present, are its description. */
export function controlLinks(id: string, hasConstraint: boolean, hasError: boolean): ControlLinks {
  const described = [hasConstraint ? `${id}-constraint` : '', hasError ? `${id}-error` : ''].filter((part) => part !== '');
  return {
    id: `${id}-control`,
    'aria-describedby': described.length > 0 ? described.join(' ') : undefined,
    'aria-invalid': hasError ? true : undefined,
  };
}
