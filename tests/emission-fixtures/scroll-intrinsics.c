#pragma bank 0
u8 value;
void main(){__scroll_bg_set(1,2);__scroll_bg_x_set(value);__scroll_bg_y_set(value);value=__scroll_bg_x_get();value=__scroll_bg_y_get();
__scroll_win_set(7,8);__scroll_win_x_set(value);__scroll_win_y_set(value);value=__scroll_win_x_get();value=__scroll_win_y_get();
__scroll_bg_set_buffered(11,12);__scroll_bg_x_set_buffered(value);__scroll_bg_y_set_buffered(value);
__scroll_win_set_buffered(17,18);__scroll_win_x_set_buffered(value);__scroll_win_y_set_buffered(value);
__scroll_flush();__scroll_bg_add(3,4);__scroll_win_add(1,2);__scroll_win_show();__scroll_win_hide();}
