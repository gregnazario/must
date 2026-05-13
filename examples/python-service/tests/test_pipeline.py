from data_pipeline import process


def test_process_filters_inactive():
    data = [
        {"id": 1, "active": True},
        {"id": 2, "active": False},
        {"id": 3},
    ]
    result = process(data)
    assert len(result) == 2
    assert result[0]["id"] == 1
    assert result[1]["id"] == 3


def test_process_empty_input():
    assert process([]) == []
