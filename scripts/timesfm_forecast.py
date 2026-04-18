#!/usr/bin/env python3
"""TimesFM forecasting bridge script for ICTMS"""

import json
import sys
import numpy as np

def load_model():
    """Load TimesFM model"""
    import torch
    import timesfm
    
    torch.set_float32_matmul_precision("high")
    
    model = timesfm.TimesFM_2p5_200M_torch.from_pretrained(
        "google/timesfm-2.5-200m-pytorch"
    )
    model.compile(timesfm.ForecastConfig(
        max_context=1024,
        max_horizon=256,
        normalize_inputs=True,
        use_continuous_quantile_head=True,
        force_flip_invariance=True,
        infer_is_positive=True,
        fix_quantile_crossing=True,
    ))
    
    return model

def forecast(model, prices, horizon):
    """Run forecast"""
    inputs = [np.array(prices, dtype=np.float32)]
    
    point_forecast, quantile_forecast = model.forecast(
        horizon=horizon,
        inputs=inputs
    )
    
    return {
        "point_forecast": point_forecast[0].tolist(),
        "quantile_forecast": quantile_forecast[0].tolist()
    }

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No input file provided"}))
        sys.exit(1)
    
    input_file = sys.argv[1]
    
    try:
        with open(input_file, 'r') as f:
            data = json.load(f)
        
        prices = data["prices"]
        horizon = data["horizon"]
        symbol = data.get("symbol", "UNKNOWN")
        
        model = load_model()
        result = forecast(model, prices, horizon)
        
        result["symbol"] = symbol
        result["horizon"] = horizon
        
        print(json.dumps(result))
        
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
