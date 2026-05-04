#define REAL float

void task_body_add(REAL* A, REAL* B, REAL* C, int m, int n, int p, int ld) {
    for (int i = 0; i < m; i++)
        for (int k = 0; k < p; k++) {
            REAL c = 0.0;
            for (int j = 0; j < n; j++)
                c += A[i * ld + j] * B[j * ld + k];
            C[i * ld + k] += c;
        }
}
void task_body_noadd(REAL* A, REAL* B, REAL* C, int m, int n, int p, int ld) {
    for (int i = 0; i < m; i++)
        for (int k = 0; k < p; k++) {
            REAL c = 0.0;
            for (int j = 0; j < n; j++)
                c += A[i * ld + j] * B[j * ld + k];
            C[i * ld + k] = c;
        }
}
