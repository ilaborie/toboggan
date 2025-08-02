
/**
 * ISO 8601 timestamp string
 */
export type Timestamp = string;

/**
 * Duration structure
 */
export type Duration = {
    secs: number;
    // XXX We don't care about nanos
    // nanos: number;
};