FROM python:3.10-slim

WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application files
COPY app.py .
COPY templates/ ./templates/
COPY static/ ./static/

# Create directories if needed
RUN mkdir -p ./logs

# Expose port
EXPOSE 5000

# Run the application
CMD ["python", "app.py"]
